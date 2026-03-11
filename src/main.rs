use std::io;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    DisableMouseCapture, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

use rodio::stream::DeviceSinkBuilder;
use rodio::{Player, Source};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use serde::{Deserialize, Serialize};
use stratum_dsp::{analyze_audio, AnalysisConfig};

const OVERVIEW_RESOLUTION: usize = 4000;
const ZOOM_LEVELS: &[f32] = &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const DEFAULT_ZOOM_IDX: usize = 2; // 4 seconds
const FADE_SAMPLES: i64 = 256;       // ~5.8ms at 44100 Hz — fade-out then fade-in around each seek
const MICRO_FADE_SAMPLES: i64 = 8;  // ~0.2ms — used for micro-jumps (c/d keys)
const DETECT_MODES: &[&str] = &["auto", "fusion", "legacy"];
/// Spectral colour palettes: (name, bass_rgb, treble_rgb).
const SPECTRAL_PALETTES: &[(&str, (u8,u8,u8), (u8,u8,u8))] = &[
    ("amber/cyan", (255, 140,   0), (  0, 200, 200)),
    ("soft",       (200, 130,  50), ( 50, 190, 200)),
    ("spectrum",   ( 80, 110, 220), (220, 200,  60)),
    ("green",      (120, 200,  60), ( 60, 200, 170)),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let arg = args.get(1).map(std::path::PathBuf::from);
    let start = arg.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

    // Set up terminal once — shared by browser and player.
    let setup = (|| -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        io::stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?
            .execute(PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES))?;
        Terminal::new(CrosstermBackend::new(io::stdout()))
    })();
    let mut terminal = match setup {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Terminal error: {e}");
            std::process::exit(1);
        }
    };

    // Load cache early so we can read last_browser_path before the browser opens.
    let cache_file = cache_path();
    let mut cache = Cache::load(cache_file);

    // Compute the initial browser directory:
    //   CLI dir arg  → that directory (overrides last-visited for this first open only)
    //   CLI file arg → the file's parent directory
    //   no arg       → last visited path from cache (if it still exists), else CWD
    let mut browser_dir: std::path::PathBuf = if arg.as_deref().map(|p| p.is_dir()).unwrap_or(false) {
        start.clone()
    } else if start.is_file() {
        start.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| start.clone())
    } else {
        cache.last_browser_path()
            .filter(|p| p.exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
    };

    // If start is a directory (or CWD), run the browser first.
    let file_path = if start.is_file() {
        start
    } else {
        match run_browser(&mut terminal, browser_dir.clone()) {
            Ok((BrowserResult::Selected(p), cwd)) => {
                browser_dir = cwd;
                cache.set_last_browser_path(&browser_dir);
                cache.save();
                p
            }
            Ok((BrowserResult::ReturnToPlayer, _)) | Ok((BrowserResult::Quit, _)) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
                return;
            }
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
                eprintln!("Browser error: {e}");
                std::process::exit(1);
            }
        }
    };

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            let _ = disable_raw_mode();
            let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();
    let mut next_path = file_path;

    loop {
        let path_str = next_path.to_string_lossy().to_string();
        let file_dir = next_path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let decoded_samples = Arc::new(AtomicUsize::new(0));
        let estimated_total = Arc::new(AtomicUsize::new(0));
        let (decode_tx, decode_rx) = mpsc::channel::<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>();
        {
            let path_clone = path_str.clone();
            let ds = Arc::clone(&decoded_samples);
            let et = Arc::clone(&estimated_total);
            thread::spawn(move || {
                let _ = decode_tx.send(decode_audio(&path_clone, ds, et).map_err(|e| e.to_string()));
            });
        }

        let decode_result = loop {
            let done = decoded_samples.load(Ordering::Relaxed);
            let total = estimated_total.load(Ordering::Relaxed);
            let ratio = if total > 0 { (done as f64 / total as f64).min(1.0) } else { 0.0 };

            let _ = terminal.draw(|frame| {
                let area = frame.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
                    .split(area);
                frame.render_widget(
                    Paragraph::new(format!("Loading {}…", path_str))
                        .style(Style::default().fg(Color::DarkGray)),
                    chunks[0],
                );
                let label = if total > 0 { format!("{:.0}%", ratio * 100.0) } else { String::new() };
                frame.render_widget(Gauge::default().ratio(ratio).label(label), chunks[1]);
            });

            if let Ok(result) = decode_rx.try_recv() {
                break result;
            }

            if event::poll(Duration::from_millis(30)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.code == KeyCode::Char('q') {
                        let _ = disable_raw_mode();
                        let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
                        return;
                    }
                }
            }
        };

        let (mono, stereo, sample_rate, channels) = match decode_result {
            Ok(v) => v,
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
                eprintln!("Decode error: {e}");
                std::process::exit(1);
            }
        };

        let filename = next_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&path_str)
            .to_string();

        let total_duration = Duration::from_secs(mono.len() as u64 / sample_rate as u64);
        let mono = Arc::new(mono);
        let waveform = Arc::new(WaveformData::compute(Arc::clone(&mono), sample_rate));

        let samples = Arc::new(stereo);
        let position = Arc::new(AtomicUsize::new(0));
        let fade_remaining = Arc::new(AtomicI64::new(0));
        let fade_len = Arc::new(AtomicI64::new(FADE_SAMPLES));
        let pending_target = Arc::new(AtomicUsize::new(usize::MAX));
        let seek_handle = SeekHandle {
            samples: Arc::clone(&samples),
            position: Arc::clone(&position),
            fade_remaining: Arc::clone(&fade_remaining),
            fade_len: Arc::clone(&fade_len),
            pending_target: Arc::clone(&pending_target),
            sample_rate,
            channels,
        };

        let player = Player::connect_new(&mixer);
        player.append(TrackingSource::new(
            samples, position, fade_remaining, fade_len, pending_target, sample_rate, channels,
        ));

        let (bpm_tx, bpm_rx) = mpsc::channel::<(String, f32, i64)>();
        {
            let mono_bg = Arc::clone(&mono);
            let entries = cache.entries_snapshot();
            thread::spawn(move || {
                let hash = hash_mono(&mono_bg);
                let (bpm, offset_ms) = if let Some(entry) = entries.get(&hash) {
                    (entry.bpm, entry.offset_ms)
                } else {
                    let bpm = detect_bpm(&mono_bg, sample_rate).map(|b| b.round()).unwrap_or(120.0);
                    (bpm, 0i64)
                };
                let _ = bpm_tx.send((hash, bpm, offset_ms));
            });
        }

        match tui_loop(
            &mut terminal,
            &filename,
            &file_dir,
            bpm_rx,
            total_duration,
            Arc::clone(&waveform),
            &player,
            &seek_handle,
            &mono,
            sample_rate,
            &mut cache,
            &mut browser_dir,
            &mixer,
        ) {
            Ok(Some(path)) => { next_path = path; }
            Ok(None) => break,
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
                eprintln!("TUI error: {e}");
                std::process::exit(1);
            }
        }
    }

    let _ = disable_raw_mode();
    let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    filename: &str,
    file_dir: &std::path::Path,
    mut bpm_rx: mpsc::Receiver<(String, f32, i64)>,
    total_duration: Duration,
    waveform: Arc<WaveformData>,
    player: &Player,
    seek_handle: &SeekHandle,
    mono: &Arc<Vec<f32>>,
    sample_rate: u32,
    cache: &mut Cache,
    browser_dir: &mut std::path::PathBuf,
    mixer: &rodio::mixer::Mixer,
) -> io::Result<Option<std::path::PathBuf>> {
    let (keymap, display_cfg) = load_config();
    let mut bpm: f32 = 120.0;
    let mut base_bpm: f32 = 120.0; // detected BPM; speed factor = bpm / base_bpm
    let mut offset_ms: i64 = 0;
    let mut analysis_hash: Option<String> = None;
    let mut frame_count: usize = 0;
    let mut last_scrub_samp: f64 = -1.0; // tracks position of last warp scrub for throttling
    let mut tap_times: Vec<f64> = Vec::new();
    let mut last_tap_wall: Option<Instant> = None;
    let mut tap_offset_pending: Option<i64> = None; // Some = tap-guided detection in flight
    let mut tap_guided_rx: bool = false;             // current bpm_rx is from a tap-guided spawn
    let mut was_tap_active: bool = false;
    // Smooth display position: advances via wall clock to avoid audio-buffer-burst jitter.
    let mut smooth_display_samp: f64 = 0.0;
    let mut last_render = Instant::now();
    let mut last_viewport_start: usize = 0;
    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let mut detect_mode: usize = 0;
    let mut detail_height: usize = 8;
    let mut overview_rect = ratatui::layout::Rect::default();
    let mut last_bar_cols: Vec<usize> = Vec::new();
    let mut nudge: i8 = 0; // -1 = backward, 0 = none, +1 = forward
    let mut nudge_mode = NudgeMode::Jump;
    let mut space_held = false;
    let mut space_chord_fired = false;
    let mut volume: f32 = 1.0;
    let mut help_open = false;
    let mut palette_idx: usize = 0;
    let mut audio_latency_ms: i64 = cache.get_latency();
    let mut calibration_mode = false;
    // Wall-clock time of the last calibration pulse fired (used for travelling marker and next-pulse scheduling).
    let mut last_calib_pulse: Option<Instant> = None;
    const CALIB_PERIOD_SECS: f64 = 1.0; // 60 BPM = 1 s period

    // Shared state for the background detail-braille thread.
    let detail_cols = Arc::new(AtomicUsize::new(0));
    let detail_rows = Arc::new(AtomicUsize::new(0));
    let detail_zoom_at = Arc::new(AtomicUsize::new(zoom_idx));
    let detail_style = Arc::new(AtomicUsize::new(0)); // 0 = fill, 1 = outline
    let detail_braille_shared: Arc<Mutex<Arc<BrailleBuffer>>> =
        Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
    let display_pos_shared = Arc::new(AtomicUsize::new(0));

    // StopOnDrop sets the stop flag when tui_loop exits for any reason.
    struct StopOnDrop(Arc<AtomicBool>);
    impl Drop for StopOnDrop {
        fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
    }
    let stop_detail = Arc::new(AtomicBool::new(false));
    let _stop_guard = StopOnDrop(Arc::clone(&stop_detail));

    {
        let cols_bg        = Arc::clone(&detail_cols);
        let rows_bg        = Arc::clone(&detail_rows);
        let zoom_bg        = Arc::clone(&detail_zoom_at);
        let style_bg       = Arc::clone(&detail_style);
        let braille_bg     = Arc::clone(&detail_braille_shared);
        let stop_bg        = Arc::clone(&stop_detail);
        let wf_bg          = Arc::clone(&waveform);

        let display_pos_bg = Arc::clone(&display_pos_shared);
        let sr_bg          = sample_rate;
        let ch_bg          = seek_handle.channels;

        thread::spawn(move || {
            let mut last_cols            = 0usize;
            let mut last_rows            = 0usize;
            let mut last_zoom            = usize::MAX;
            let mut last_style           = usize::MAX;
            let mut last_anchor_sample   = 0usize;
            let mut last_samples_per_col = 0usize;

            loop {
                if stop_bg.load(Ordering::Relaxed) { break; }

                let cols = cols_bg.load(Ordering::Relaxed);
                let rows = rows_bg.load(Ordering::Relaxed);
                if cols == 0 || rows == 0 {
                    thread::sleep(Duration::from_millis(8));
                    continue;
                }

                let zoom         = zoom_bg.load(Ordering::Relaxed).min(ZOOM_LEVELS.len() - 1);
                let zoom_secs    = ZOOM_LEVELS[zoom];
                // Always use the smooth display position as the buffer centre.
                // Using the raw audio position causes premature recomputes and off-centre
                // buffers whenever rodio bursts the audio position forward.
                let pos_samp     = display_pos_bg.load(Ordering::Relaxed) / ch_bg as usize;
                let col_samp     = ((zoom_secs * sr_bg as f32) as usize / cols).max(1);

                // Recompute when dimensions/zoom change or the viewport approaches the buffer edge.
                let drift_cols = if last_samples_per_col > 0 {
                    let drift = pos_samp as i64 - last_anchor_sample as i64;
                    drift.unsigned_abs() as usize / last_samples_per_col
                } else {
                    usize::MAX // force initial compute
                };

                let style = style_bg.load(Ordering::Relaxed);
                let must_recompute = cols != last_cols || rows != last_rows || zoom != last_zoom || style != last_style || drift_cols >= cols * 3 / 4;

                if must_recompute {
                    let buf_cols = cols * 5;
                    // Align anchor to the column grid so every buffer at this zoom level shares
                    // the same column boundaries — overlapping columns are byte-for-byte identical,
                    // making the buffer handoff visually seamless.
                    let anchor = (pos_samp / col_samp) * col_samp;
                    let mono   = &wf_bg.mono;

                    let peaks: Vec<(f32, f32)> = (0..buf_cols).map(|c| {
                        let offset    = c as i64 - (buf_cols / 2) as i64;
                        let raw_start = anchor as i64 + offset * col_samp as i64;
                        if raw_start < 0 {
                            return (1.0, -1.0); // before track start — render as blank
                        }
                        let samp_start = raw_start as usize;
                        let samp_end   = (samp_start + col_samp).min(mono.len());
                        if samp_start >= mono.len() {
                            return (0.0, 0.0);
                        }
                        let chunk = &mono[samp_start..samp_end];
                        let mn = chunk.iter().cloned().fold(f32::INFINITY,     f32::min);
                        let mx = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        (mn.max(-1.0), mx.min(1.0))
                    }).collect();

                    *braille_bg.lock().unwrap() = Arc::new(BrailleBuffer {
                        grid: render_braille(&peaks, rows, buf_cols, style == 1),
                        buf_cols,
                        anchor_sample:   anchor,
                        samples_per_col: col_samp,
                    });
                    last_cols            = cols;
                    last_rows            = rows;
                    last_zoom            = zoom;
                    last_style           = style;
                    last_anchor_sample   = anchor;
                    last_samples_per_col = col_samp;
                }

                thread::sleep(Duration::from_millis(8));
            }
        });
    }

    loop {
        frame_count += 1;
        if let Ok((hash, new_bpm, new_offset)) = bpm_rx.try_recv() {
            if tap_guided_rx {
                tap_guided_rx = false;
                match tap_offset_pending.take() {
                    Some(tap_offset) => {
                        // Tap-guided result arrived before tap reset: use analyser BPM, preserve tap phase.
                        let speed_ratio = bpm / base_bpm;
                        base_bpm = new_bpm;
                        bpm = (base_bpm * speed_ratio).clamp(40.0, 240.0);
                        offset_ms = tap_offset;
                        player.set_speed(bpm / base_bpm);
                        cache.set(hash.clone(), CacheEntry { bpm: base_bpm, offset_ms, name: filename.to_string() });
                        cache.save();
                        analysis_hash = Some(hash);
                    }
                    None => {
                        // Tap session reset before result arrived — discard entirely.
                        // Restore analysis_hash so the UI stops showing the spinner.
                        analysis_hash = Some(hash);
                    }
                }
            } else {
                bpm = new_bpm;
                base_bpm = new_bpm;
                offset_ms = new_offset;
                cache.set(hash.clone(), CacheEntry { bpm, offset_ms, name: filename.to_string() });
                cache.save();
                analysis_hash = Some(hash);
            }
        }

        // Snapshot samples_per_col for use in scrub outside the draw closure.
        let scrub_spc = detail_braille_shared.lock().unwrap().samples_per_col;

        let now = Instant::now();
        let dc = detail_cols.load(Ordering::Relaxed);
        let zoom_secs = ZOOM_LEVELS[zoom_idx];
        let col_secs = if dc > 0 { zoom_secs as f64 / dc as f64 } else { 0.033 };
        let elapsed = now.duration_since(last_render).as_secs_f64()
            // Cap at 4 columns per frame. Must exceed the minimum poll_dur (8ms) at every zoom
            // level — a tighter cap causes systematic drift and periodic large-drift snapping.
            .min(col_secs * 4.0);
        last_render = now;

        // Real audio position — used for beat flash, time display, overview playhead.
        let pos_raw  = seek_handle.position.load(Ordering::Relaxed);
        let pos_samp = pos_raw / seek_handle.channels as usize;
        let pos      = Duration::from_secs_f64(pos_samp as f64 / sample_rate as f64);

        // Smooth display position — advances via wall clock to avoid audio-buffer-burst jitter.
        // Large drift (seek / startup) snaps immediately. Small drift correction rate is 1.0
        // (also snaps) so any firing is visually obvious — for empirical observation only.
        if !player.is_paused() {
            let speed = (bpm / base_bpm) as f64 * (1.0 + nudge as f64 * 0.1);
            smooth_display_samp += elapsed * sample_rate as f64 * speed;
        } else if nudge != 0 {
            // While paused and nudging, drift the display position at ±10% of normal speed
            // and sync the actual position atomic so seeking remains accurate.
            let total_mono_samps =
                (seek_handle.samples.len() / seek_handle.channels as usize) as f64;
            smooth_display_samp = (smooth_display_samp
                + elapsed * sample_rate as f64 * nudge as f64 * 0.1)
                .clamp(0.0, total_mono_samps);
            // Use set_position (no quiet-frame search) — no audio is playing so no click occurs.
            seek_handle.set_position(smooth_display_samp / sample_rate as f64);
            // Scrub: fire a snippet once per half-column advance for smooth continuity.
            let half_spc = (scrub_spc / 2).max(1);
            if scrub_spc > 0
                && (smooth_display_samp - last_scrub_samp).abs() >= half_spc as f64
            {
                scrub_audio(mixer, &seek_handle.samples, seek_handle.channels as u16,
                            sample_rate, smooth_display_samp as usize, half_spc);
                last_scrub_samp = smooth_display_samp;
            }
        }
        let drift = smooth_display_samp - pos_samp as f64;
        // When paused there is no audio jitter, so snap on any non-trivial drift (e.g. after
        // a beat jump while paused). When playing, only snap on large drift (>500ms) to avoid
        // reacting to normal audio-burst jitter.
        if drift.abs() > sample_rate as f64 * 0.5 || (player.is_paused() && nudge == 0 && drift.abs() > 1.0) {
            // Large drift (seek / startup) — snap to the nearest column boundary so
            // sub_col=false after every seek. This prevents sub_col from alternating
            // when the seek distance is an odd number of half-columns, which would
            // oscillate the viewport by 0.5 columns on every other seek.
            let col_samp_f64 = col_secs * sample_rate as f64;
            smooth_display_samp = if col_samp_f64 > 0.0 {
                (pos_samp as f64 / col_samp_f64).round() * col_samp_f64
            } else {
                pos_samp as f64
            };
        }
        // Apply audio latency compensation: shift the visual display backward by latency.
        let display_samp = smooth_display_samp
            - audio_latency_ms as f64 * sample_rate as f64 / 1000.0;
        let display_pos_samp = display_samp.max(0.0) as usize;
        display_pos_shared.store(display_pos_samp * seek_handle.channels as usize, Ordering::Relaxed);

        // Beat-derived values — recomputed each frame so they react to bpm changes instantly.
        // Use base_bpm and display_samp so the flash is in exact sync with tick marks.
        let beat_period = Duration::from_secs_f64(60.0 / base_bpm as f64);
        let flash_window = beat_period.mul_f64(0.15);

        // Beat flash — subtract offset so phase==0 when cursor is on a tick.
        // Ticks are at (samp - offset_samp) % beat_period_samp == 0, so we subtract here.
        let smooth_pos_ns = (display_samp / sample_rate as f64 * 1_000_000_000.0) as i128
            - offset_ms as i128 * 1_000_000;
        let phase = smooth_pos_ns.rem_euclid(beat_period.as_nanos() as i128);
        let beat_on = phase < flash_window.as_nanos() as i128;

        let remaining = total_duration.saturating_sub(pos);
        let warning_active = !player.is_paused()
            && remaining < Duration::from_secs_f32(display_cfg.warning_threshold_secs);
        let beat_index = smooth_pos_ns.div_euclid(beat_period.as_nanos() as i128);
        let warn_beat_on = warning_active && (beat_index % 2 == 0);

        let analysing = analysis_hash.is_none();

        // Calibration pulse: fire a click tone at 120 BPM (every 0.5 s) while calibration_mode active.
        if calibration_mode {
            let fire = match last_calib_pulse {
                None => true,
                Some(t) => t.elapsed().as_secs_f64() >= CALIB_PERIOD_SECS,
            };
            if fire {
                play_click_tone(mixer, sample_rate);
                last_calib_pulse = Some(Instant::now());
            }
        } else {
            last_calib_pulse = None;
        }

        // Detect tap session end: active last frame, now timed out.
        let tap_active_now = !tap_times.is_empty()
            && last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
        if was_tap_active && !tap_active_now && tap_times.len() >= 8 {
            if let Some(ref hash) = analysis_hash {
                let (tapped_bpm, tapped_offset) = compute_tap_bpm_offset(&tap_times);
                let config = AnalysisConfig {
                    force_legacy_bpm: true,
                    bpm_resolution: 0.1,
                    min_bpm: tapped_bpm * 0.95,
                    max_bpm: tapped_bpm * 1.05,
                    legacy_bpm_preferred_min: tapped_bpm * 0.95,
                    legacy_bpm_preferred_max: tapped_bpm * 1.05,
                    ..AnalysisConfig::default()
                };
                let hash = hash.clone();
                let mono_bg = Arc::clone(mono);
                let (tx, rx) = mpsc::channel::<(String, f32, i64)>();
                let offset_snap = tapped_offset;
                thread::spawn(move || {
                    if let Ok(result) = analyze_audio(&mono_bg, sample_rate, config) {
                        let _ = tx.send((hash, result.bpm, offset_snap));
                    }
                });
                bpm_rx = rx;
                analysis_hash = None;
                tap_offset_pending = Some(tapped_offset);
                tap_guided_rx = true;
            }
        }
        was_tap_active = tap_active_now;

        let fmt_dur = |d: Duration| {
            let s = d.as_secs();
            format!("{:02}:{:02}", s / 60, s % 60)
        };
        let time_str = format!("{} / {}", fmt_dur(pos), fmt_dur(total_duration));
        detail_zoom_at.store(zoom_idx, Ordering::Relaxed);

        terminal.draw(|frame| {
            let area = frame.area();

            let outer = Block::default()
                .title(format!(" tj {} — {filename} ", env!("CARGO_PKG_VERSION")))
                .borders(Borders::ALL);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // info bar
                    Constraint::Length(5), // overview waveform
                    Constraint::Min(0),    // detail waveform + blank space
                ])
                .split(inner);
            // Sub-split the detail area: fixed height + blank space below.
            let detail_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(detail_height as u16),
                    Constraint::Min(0),
                ])
                .split(chunks[2])[0];

            // Info bar
            {
                let play_icon = if player.is_paused() { "⏸" } else { "▶" };
                let mode_str = match nudge_mode {
                    NudgeMode::Jump => "jump",
                    NudgeMode::Warp => "warp",
                };
                let nudge_str = match nudge {
                    1  => "  ▶nudge",
                    -1 => "  ◀nudge",
                    _  => "",
                };
                let tap_active = !tap_times.is_empty()
                    && last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
                let tap_str = if tap_active {
                    format!("  tap:{}", tap_times.len())
                } else {
                    String::new()
                };
                let dim = Style::default().fg(Color::DarkGray);
                let info_line = if analysing {
                    Line::from(vec![
                        Span::styled(format!("{play_icon}  "), dim),
                        Span::styled(
                            format!("[analysing {}]", SPINNER[frame_count % SPINNER.len()]),
                            dim,
                        ),
                    ])
                } else {
                    let beat_style = if beat_on {
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::Rgb(60, 50, 0))
                    } else {
                        dim
                    };
                    let adjusted = (bpm - base_bpm).abs() >= 0.05;
                    let mut info_spans = vec![
                        Span::styled(format!("{play_icon}  "), dim),
                    ];
                    if adjusted {
                        info_spans.push(Span::styled(format!("{:.2} ", base_bpm), dim));
                        info_spans.push(Span::styled("(", dim));
                        info_spans.push(Span::styled(format!("{:.2}", bpm), beat_style));
                        info_spans.push(Span::styled(")", dim));
                    } else {
                        info_spans.push(Span::styled(format!("{:.2}", base_bpm), beat_style));
                    }
                    let lat_str = if calibration_mode {
                        format!("  lat:{}ms  ~ to exit", audio_latency_ms)
                    } else {
                        String::new()
                    };
                    info_spans.push(Span::styled(
                        format!("  {:+}ms  {}s  vol:{}%{}  nudge:{}{}  {}  [?]{}",
                            offset_ms, zoom_secs,
                            (volume * 100.0).round() as u32, nudge_str,
                            mode_str, tap_str,
                            SPECTRAL_PALETTES[palette_idx].0, lat_str),
                        dim,
                    ));
                    Line::from(info_spans)
                };
                frame.render_widget(Paragraph::new(info_line), chunks[0]);
            }

            // Overview waveform — Braille rendered fresh each frame (O(cols×rows), negligible).
            let ow = chunks[1].width as usize;
            let oh = chunks[1].height as usize;
            let total_peaks = waveform.peaks.len();
            let playhead_frac = if total_duration.is_zero() {
                0.0
            } else {
                (display_samp / sample_rate as f64 / total_duration.as_secs_f64()).clamp(0.0, 1.0)
            };
            let playhead_col = ((playhead_frac * ow as f64) as usize).min(ow.saturating_sub(1));

            let (ov_peaks, ov_bass): (Vec<(f32, f32)>, Vec<f32>) = (0..ow)
                .map(|col| {
                    let idx = (col * total_peaks / ow.max(1)).min(total_peaks.saturating_sub(1));
                    (waveform.peaks[idx], waveform.bass_ratio[idx])
                })
                .unzip();
            let ov_braille = render_braille(&ov_peaks, oh, ow, false);
            let (bar_cols, bars_per_tick): (Vec<usize>, u32) = if !analysing {
                bar_tick_cols(base_bpm as f64, offset_ms, total_duration.as_secs_f64(), ow)
            } else {
                (Vec::new(), 4)
            };
            overview_rect = chunks[1];
            last_bar_cols = bar_cols.clone();
            let legend: String = if !analysing {
                format!("{} bars", bars_per_tick)
            } else {
                String::new()
            };
            let legend_start = ow.saturating_sub(legend.len());

            let ov_lines: Vec<Line<'static>> = ov_braille
                .into_iter()
                .enumerate()
                .map(|(r, row)| {
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut run = String::new();
                    let mut run_color = Color::Reset;
                    for (c, byte) in row.into_iter().enumerate() {
                        // Row 0 top-right: legend overlay takes priority over everything.
                        let (color, ch) = if r == 0 && c >= legend_start && !legend.is_empty() {
                            let lch = legend.chars().nth(c - legend_start).unwrap_or(' ');
                            (Color::DarkGray, lch)
                        } else if c == playhead_col {
                            (Color::White, '\u{28FF}') // ⣿ solid playhead
                        } else if bar_cols.contains(&c) && (r == 0 || r + 1 == oh) {
                            if warn_beat_on {
                                (Color::Rgb(120, 60, 60), '\u{28FF}')
                            } else if warning_active {
                                (Color::Rgb(40, 20, 20), '\u{28FF}')
                            } else {
                                (Color::DarkGray, '\u{28FF}') // ⣿ tick at top and bottom rows only
                            }
                        } else {
                            let r = ov_bass[c];
                            let (_, (br, bg, bb), (tr, tg, tb)) = SPECTRAL_PALETTES[palette_idx];
                            let spectral = Color::Rgb(
                                (br as f32 * r + tr as f32 * (1.0 - r)) as u8,
                                (bg as f32 * r + tg as f32 * (1.0 - r)) as u8,
                                (bb as f32 * r + tb as f32 * (1.0 - r)) as u8,
                            );
                            (spectral, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                        };
                        if color != run_color {
                            if !run.is_empty() {
                                spans.push(Span::styled(
                                    std::mem::take(&mut run),
                                    Style::default().fg(run_color),
                                ));
                            }
                            run_color = color;
                        }
                        run.push(ch);
                    }
                    if !run.is_empty() {
                        spans.push(Span::styled(run, Style::default().fg(run_color)));
                    }
                    Line::from(spans)
                })
                .collect();

            frame.render_widget(Paragraph::new(ov_lines), chunks[1]);

            // Detail waveform — pan a viewport through the stable background-thread buffer.
            let dw = detail_area.width as usize;
            let dh = detail_area.height as usize;
            detail_cols.store(dw, Ordering::Relaxed);
            // Reserve top and bottom rows for tick marks; waveform uses the inner rows.
            let waveform_rows = dh.saturating_sub(2);
            detail_rows.store(waveform_rows, Ordering::Relaxed);
            let buf = Arc::clone(&*detail_braille_shared.lock().unwrap());
            let centre_col = if calibration_mode {
                dw / 2
            } else {
                ((dw as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
                    .clamp(0, dw.saturating_sub(1))
            };

            // Compute the column offset into the buffer that places the playhead at centre_col.
            // Uses smooth_display_samp (wall-clock based) to avoid audio-buffer-burst jitter.
            // delta_half and half_col_samp are hoisted out so the tick renderer can use the
            // same quantized visual centre as the waveform (they share an identical reference).
            let mut sub_col = false;
            let mut delta_half_global: i64 = 0;
            let mut half_col_samp_global: f64 = 1.0;
            let viewport_start: Option<usize> = if buf.buf_cols >= dw && buf.samples_per_col > 0 {
                let delta = display_pos_samp as i64 - buf.anchor_sample as i64;
                let half_col_samp = buf.samples_per_col as f64 / 2.0;
                let delta_half = (delta as f64 / half_col_samp).round() as i64;
                sub_col = delta_half % 2 != 0;
                delta_half_global = delta_half;
                half_col_samp_global = half_col_samp;
                // div_euclid gives floor division so column-step phase is symmetric across
                // positive and negative delta (vs advances once per full column, consistently).
                let delta_cols = delta_half.div_euclid(2);
                let vs = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
                // Need dw+1 columns when sub_col to supply the extra byte for the shift.
                let need = if sub_col { dw + 1 } else { dw };
                if vs >= 0 && (vs as usize) + need <= buf.buf_cols {
                    let v = vs as usize;
                    last_viewport_start = v;
                    Some(v)
                } else {
                    // vs out of bounds means either a seek just happened or the buffer hasn't
                    // been recomputed yet. Show blank — the background thread recomputes at 75%
                    // capacity, so in normal playback vs never reaches the buffer edges.
                    None
                }
            } else {
                None
            };

            // Compute tick marks directly in display space at half-column resolution.
            // Uses the same quantized visual centre as the waveform (anchor + delta_half * half_spc)
            // so ticks and waveform step in exact lock-step.
            // Each beat: disp_half = round((t_samp - view_start) / half_spc);
            //   even → left-half tick  (0x47 = ⡇), odd → right-half tick (0xB8 = ⢸).
            let tick_display: Vec<u8> = if !analysing && buf.samples_per_col > 0 {
                let mut row = vec![0u8; dw];
                let spc = buf.samples_per_col as f64;
                let half_spc = half_col_samp_global;
                let beat_period_samp = 60.0 / base_bpm as f64 * sample_rate as f64;
                let offset_samp = offset_ms as f64 / 1000.0 * sample_rate as f64;
                // Quantized visual centre: same reference point the waveform uses.
                let visual_centre = buf.anchor_sample as f64 + delta_half_global as f64 * half_spc;
                let view_start = visual_centre - centre_col as f64 * spc;
                let view_end = view_start + dw as f64 * spc;
                let n_start = ((view_start - offset_samp) / beat_period_samp).floor() as i64 - 1;
                let mut t_samp = offset_samp + n_start as f64 * beat_period_samp;
                while t_samp <= view_end {
                    let disp_half = ((t_samp - view_start) / half_spc).round() as i64;
                    if disp_half >= 0 {
                        let col = (disp_half / 2) as usize;
                        if col < dw {
                            row[col] = if disp_half % 2 != 0 { 0xB8 } else { 0x47 };
                        }
                    }
                    t_samp += beat_period_samp;
                }
                row
            } else {
                vec![]
            };

            // Calibration pulse markers — same grid logic as beat ticks but at 60 BPM.
            // Anchored to smooth_display_samp (audio position) so that increasing audio_latency_ms
            // makes the marker appear further ahead of the playhead at click-fire time, reaching
            // the playhead later — at the moment the click is actually heard.
            let (calib_display, calib_pulse_on_playhead): (Vec<u8>, bool) =
                if calibration_mode && buf.samples_per_col > 0 {
                    let spc = buf.samples_per_col as f64;
                    let half_spc = half_col_samp_global;
                    let speed = (bpm / base_bpm) as f64;
                    let calib_period_samp = CALIB_PERIOD_SECS * sample_rate as f64 * speed;
                    let elapsed_since_pulse = last_calib_pulse
                        .map(|t| t.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                    // Anchor to smooth_display_samp (audio position), NOT display_samp.
                    // This means the marker appears audio_latency_ms ms ahead of the playhead
                    // at click-fire time, and travels to the playhead over that many ms.
                    let pulse_origin_samp = smooth_display_samp
                        - elapsed_since_pulse * sample_rate as f64 * speed;
                    let visual_centre = buf.anchor_sample as f64 + delta_half_global as f64 * half_spc;
                    let view_start = visual_centre - centre_col as f64 * spc;
                    let view_end = view_start + dw as f64 * spc;
                    let n_start = ((view_start - pulse_origin_samp) / calib_period_samp).floor() as i64 - 1;
                    let mut row = vec![0u8; dw];
                    let mut nearest_dist_half = i64::MAX;
                    let mut t_samp = pulse_origin_samp + n_start as f64 * calib_period_samp;
                    while t_samp <= view_end {
                        let disp_half = ((t_samp - view_start) / half_spc).round() as i64;
                        if disp_half >= 0 {
                            let col = (disp_half / 2) as usize;
                            if col < dw {
                                row[col] = 0xFF; // full-column double tick
                            }
                        }
                        // Distance from this pulse to the playhead centre in half-columns.
                        let centre_half = centre_col as i64 * 2;
                        let dist = (disp_half - centre_half).abs();
                        if dist < nearest_dist_half { nearest_dist_half = dist; }
                        t_samp += calib_period_samp;
                    }
                    let flash = nearest_dist_half <= 4; // ~2 columns wide for easy visibility
                    (row, flash)
                } else {
                    (vec![], false)
                };

            // Latency position indicator: anchored to screen centre so it can travel
            // equally in both directions. ±500ms maps to ±dw/4 columns (~25ms per column),
            // so it doesn't jump on every 10ms step.
            let latency_col = (dw as i64 / 2
                + (audio_latency_ms as f64 / 500.0 * (dw as f64 / 4.0)).round() as i64)
                .clamp(0, dw as i64 - 1) as usize;

            let detail_lines: Vec<Line<'static>> = (0..dh)
                .map(|r| {
                    let is_tick_row = r == 0 || r + 1 == dh;
                    // Tick rows: computed directly in display space — no viewport slice or
                    // shift_braille_half needed. Waveform rows: apply shift_braille_half when
                    // sub_col for smooth half-column scrolling.
                    // `shifted` must be declared here so it outlives `row_slice`.
                    let shifted: Option<Vec<u8>>;
                    let row_slice: Option<&[u8]>;
                    if is_tick_row {
                        shifted = None;
                        row_slice = if tick_display.len() == dw { Some(&tick_display) } else { None };
                    } else {
                        let buf_r = r - 1;
                        shifted = if sub_col {
                            viewport_start.and_then(|vs| {
                                buf.grid.get(buf_r).map(|row| {
                                    (0..dw).map(|c| shift_braille_half(row[vs + c], row[vs + c + 1])).collect()
                                })
                            })
                        } else { None };
                        row_slice = if sub_col {
                            shifted.as_deref()
                        } else {
                            viewport_start.and_then(|vs| buf.grid.get(buf_r).map(|row| &row[vs..vs + dw]))
                        };
                    }
                    let _ = &shifted; // lifetime anchor — keeps shifted alive until row_slice is done
                    // In calibration mode: blank waveform rows; tick rows show only pulse markers.
                    if calibration_mode {
                        let mut spans: Vec<Span<'static>> = Vec::new();
                        let mut run = String::new();
                        let mut run_color = Color::Reset;
                        for c in 0..dw {
                            let calib_byte = calib_display.get(c).copied().unwrap_or(0);
                            let (color, ch) = if c == centre_col {
                                let playhead_color = if calib_pulse_on_playhead {
                                    Color::LightRed
                                } else {
                                    Color::White
                                };
                                (playhead_color, '\u{28FF}')
                            } else if is_tick_row && calib_byte != 0 {
                                (Color::Cyan, char::from_u32(0x2800 | calib_byte as u32).unwrap_or(' '))
                            } else if c == latency_col {
                                (Color::DarkGray, '\u{2502}') // │ thin latency position indicator
                            } else {
                                (Color::Reset, '\u{2800}') // blank
                            };
                            if color != run_color {
                                if !run.is_empty() {
                                    spans.push(Span::styled(
                                        std::mem::take(&mut run),
                                        Style::default().fg(run_color),
                                    ));
                                }
                                run_color = color;
                            }
                            run.push(ch);
                        }
                        if !run.is_empty() {
                            spans.push(Span::styled(run, Style::default().fg(run_color)));
                        }
                        return Line::from(spans);
                    }

                    let row = match row_slice {
                        None => return Line::from(Span::raw("\u{2800}".repeat(dw))),
                        Some(s) => s,
                    };
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut run = String::new();
                    let mut run_color = Color::Reset;
                    for (c, &byte) in row.iter().enumerate() {
                        let (color, ch) = if c == centre_col {
                            (Color::White, '\u{28FF}') // ⣿ solid centre line
                        } else if is_tick_row {
                            (Color::Gray, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                        } else {
                            (Color::Cyan, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                        };
                        if color != run_color {
                            if !run.is_empty() {
                                spans.push(Span::styled(
                                    std::mem::take(&mut run),
                                    Style::default().fg(run_color),
                                ));
                            }
                            run_color = color;
                        }
                        run.push(ch);
                    }
                    if !run.is_empty() {
                        spans.push(Span::styled(run, Style::default().fg(run_color)));
                    }
                    Line::from(spans)
                })
                .collect();

            frame.render_widget(Paragraph::new(detail_lines), detail_area);

            // Help popup
            if help_open {
                const HELP: &str = "\
Space+Z        play / pause
Space+F/V      reset tempo to detected BPM
1/2/3/4        beat jump forward 1/4/16/64 beats
q/w/e/r        beat jump backward 1/4/16/64 beats
c  /  d        nudge backward / forward (mode-dependent)
C  /  D        toggle nudge mode: jump (10ms) / warp (±10% speed)
+  /  -        beat phase offset ±10ms (or latency in calibration mode)
~              toggle latency calibration mode
z  /  Z        zoom out / in
{  /  }        detail height decrease / increase
h  /  H        BPM ½ / ×2
f  /  v        BPM +0.1 / -0.1
b              tap in time to set BPM + phase
t              re-run BPM detection
↑  /  ↓        volume up / down (5% steps)
p              cycle spectral colour palette
o              toggle waveform fill / outline
Space+A        open file browser
?              toggle this help
Esc            quit";
                let popup_w = 48u16;
                let popup_h = HELP.lines().count() as u16 + 2;
                let px = area.x + area.width.saturating_sub(popup_w) / 2;
                let py = area.y + area.height.saturating_sub(popup_h) / 2;
                let popup_rect = ratatui::layout::Rect {
                    x: px, y: py,
                    width: popup_w.min(area.width),
                    height: popup_h.min(area.height),
                };
                frame.render_widget(ratatui::widgets::Clear, popup_rect);
                frame.render_widget(
                    Paragraph::new(HELP)
                        .block(Block::default().borders(Borders::ALL).title(" Key Bindings "))
                        .style(Style::default().fg(Color::White)),
                    popup_rect,
                );
            }
        })?;

        // Adaptive frame rate: target one dot-column (half a character) per frame.
        let poll_dur = if dc > 0 {
            Duration::from_secs_f32(zoom_secs / dc as f32 / 2.0)
                .max(Duration::from_millis(8))
                .min(Duration::from_millis(200))
        } else {
            Duration::from_millis(30)
        };

        if event::poll(poll_dur)? {
            match event::read()? {
            Event::Mouse(mouse_event) => {
                if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                    let col = mouse_event.column as usize;
                    let row = mouse_event.row as usize;
                    let rect = overview_rect;
                    if col >= rect.x as usize
                        && col < (rect.x + rect.width) as usize
                        && row >= rect.y as usize
                        && row < (rect.y + rect.height) as usize
                    {
                        let ow = rect.width as usize;
                        let click_col = col - rect.x as usize;
                        let target_col = last_bar_cols.iter().copied().filter(|&c| c <= click_col).last();
                        let target_secs = match target_col {
                            Some(c) => c as f64 / ow as f64 * total_duration.as_secs_f64(),
                            None => 0.0,
                        };
                        if player.is_paused() {
                            seek_handle.seek_direct(target_secs);
                        } else {
                            seek_handle.seek_to(target_secs);
                        }
                    }
                }
            }
            Event::Key(key) => {
                // Ctrl-C: unconditional quit.
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    player.stop();
                    if let Some(ref hash) = analysis_hash {
                        if let Some(entry) = cache.get(hash.as_str()).cloned() {
                            cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                            cache.save();
                        }
                    }
                    return Ok(None);
                }
                // Space modifier: track held state for chords.
                if key.code == KeyCode::Char(' ') {
                    match key.kind {
                        KeyEventKind::Press  => { space_held = true; space_chord_fired = false; }
                        KeyEventKind::Release => { space_held = false; }
                        _ => {}
                    }
                }
                // Nudge/toggle: handled for all key kinds so Release is detected.
                match key.kind {
                    KeyEventKind::Press if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeModeToggle) => {
                        if nudge != 0 {
                            nudge = 0;
                            player.set_speed(bpm / base_bpm);
                        }
                        nudge_mode = match nudge_mode {
                            NudgeMode::Jump => NudgeMode::Warp,
                            NudgeMode::Warp => NudgeMode::Jump,
                        };
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeBackward) =>
                    {
                        match nudge_mode {
                            NudgeMode::Jump => {
                                let current = seek_handle.current_pos().as_secs_f64();
                                let target = (current - 0.010).max(0.0);
                                seek_handle.set_position(target);
                                smooth_display_samp += (target - current) * sample_rate as f64;
                                if player.is_paused() {
                                    scrub_audio(mixer, &seek_handle.samples, seek_handle.channels as u16,
                                                sample_rate, smooth_display_samp as usize, scrub_spc);
                                }
                            }
                            NudgeMode::Warp => {
                                nudge = -1;
                                player.set_speed(bpm / base_bpm * 0.9);
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeForward) =>
                    {
                        match nudge_mode {
                            NudgeMode::Jump => {
                                let current = seek_handle.current_pos().as_secs_f64();
                                let target = (current + 0.010).min(total_duration.as_secs_f64());
                                seek_handle.set_position(target);
                                smooth_display_samp += (target - current) * sample_rate as f64;
                                if player.is_paused() {
                                    scrub_audio(mixer, &seek_handle.samples, seek_handle.channels as u16,
                                                sample_rate, smooth_display_samp as usize, scrub_spc);
                                }
                            }
                            NudgeMode::Warp => {
                                nudge = 1;
                                player.set_speed(bpm / base_bpm * 1.1);
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::NudgeBackward) | Some(&Action::NudgeForward)) =>
                    {
                        if nudge_mode == NudgeMode::Warp {
                            nudge = 0;
                            player.set_speed(bpm / base_bpm);
                        }
                    }
                    _ => {}
                }
                // While help is open, any key dismisses it.
                if help_open {
                    if key.kind == KeyEventKind::Press {
                        help_open = false;
                    }
                    continue;
                }
                // All other actions fire on Press only.
                if key.kind == KeyEventKind::Press {
                    let action = if space_held && key.code != KeyCode::Char(' ') {
                        if let Some(a) = keymap.get(&KeyBinding::SpaceChord(key.code)) {
                            space_chord_fired = true;
                            space_held = false; // reset so subsequent keys aren't treated as chords
                            Some(a)
                        } else {
                            keymap.get(&KeyBinding::Key(key.code))
                        }
                    } else {
                        keymap.get(&KeyBinding::Key(key.code))
                    };
                    match action {
                    Some(Action::Quit) => {
                        player.stop();
                        cache.set_latency(audio_latency_ms);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                            }
                        }
                        cache.save();
                        return Ok(None);
                    }
                    Some(Action::OpenBrowser) => {
                        match run_browser(terminal, browser_dir.clone())? {
                            (BrowserResult::ReturnToPlayer, cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                cache.save();
                            }
                            (BrowserResult::Selected(path), cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                player.stop();
                                if let Some(ref hash) = analysis_hash {
                                    if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                        cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                                    }
                                }
                                cache.save();
                                return Ok(Some(path));
                            }
                            (BrowserResult::Quit, cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                player.stop();
                                if let Some(ref hash) = analysis_hash {
                                    if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                        cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                                    }
                                }
                                cache.save();
                                return Ok(None);
                            }
                        }
                    }
                    Some(Action::PlayPause) => {
                        if player.is_paused() { player.play(); } else { player.pause(); }
                    }
                    Some(Action::VolumeUp) => {
                        volume = (volume + 0.05).min(1.0);
                        player.set_volume(volume);
                    }
                    Some(Action::VolumeDown) => {
                        volume = (volume - 0.05).max(0.0);
                        player.set_volume(volume);
                    }
                    Some(Action::Help) => { help_open = true; }
                    Some(Action::CalibrationToggle) => {
                        if calibration_mode {
                            // Always allow exiting calibration.
                            calibration_mode = false;
                            cache.set_latency(audio_latency_ms);
                            cache.save();
                        } else if player.is_paused() {
                            // Snap to nearest 10ms so +/- steps always land on multiples of 10.
                            audio_latency_ms = (audio_latency_ms as f64 / 10.0).round() as i64 * 10;
                            calibration_mode = true;
                        }
                    }
                    Some(Action::WaveformStyle) => {
                        let s = detail_style.load(Ordering::Relaxed);
                        detail_style.store(1 - s, Ordering::Relaxed);
                    }
                    Some(Action::PaletteCycle) => {
                        palette_idx = (palette_idx + 1) % SPECTRAL_PALETTES.len();
                    }
                    Some(Action::OffsetIncrease) => {
                        if calibration_mode {
                            audio_latency_ms = (audio_latency_ms + 10 + 500).rem_euclid(1000) - 500;
                            cache.set_latency(audio_latency_ms);
                            cache.save();
                        } else {
                            offset_ms += 10;
                        }
                    }
                    Some(Action::OffsetDecrease) => {
                        if calibration_mode {
                            audio_latency_ms = (audio_latency_ms - 10 + 500).rem_euclid(1000) - 500;
                            cache.set_latency(audio_latency_ms);
                            cache.save();
                        } else {
                            offset_ms -= 10;
                        }
                    }
                    Some(Action::ZoomOut) => {
                        if zoom_idx > 0 { zoom_idx -= 1; }
                    }
                    Some(Action::ZoomIn) => {
                        if zoom_idx + 1 < ZOOM_LEVELS.len() { zoom_idx += 1; }
                    }
                    Some(Action::HeightDecrease) => {
                        if detail_height > 1 { detail_height -= 1; }
                    }
                    Some(Action::HeightIncrease) => {
                        detail_height += 1;
                    }
                    Some(Action::BpmHalve) => {
                        bpm = (bpm * 0.5).max(40.0);
                        base_bpm = bpm;
                        player.set_speed(1.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    Some(Action::BpmDouble) => {
                        bpm = (bpm * 2.0).min(240.0);
                        base_bpm = bpm;
                        player.set_speed(1.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    Some(Action::BpmIncrease) => {
                        bpm = (bpm + 0.1).min(240.0);
                        player.set_speed(bpm / base_bpm);
                    }
                    Some(Action::BpmDecrease) => {
                        bpm = (bpm - 0.1).max(40.0);
                        player.set_speed(bpm / base_bpm);
                    }
                    Some(Action::BaseBpmIncrease) => {
                        base_bpm = (base_bpm + 0.01).min(240.0);
                        bpm = base_bpm;
                        player.set_speed(1.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm: base_bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    Some(Action::BaseBpmDecrease) => {
                        base_bpm = (base_bpm - 0.01).max(40.0);
                        bpm = base_bpm;
                        player.set_speed(1.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm: base_bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    Some(Action::BpmRedetect) => {
                        if let Some(ref hash) = analysis_hash {
                            let hash = hash.clone();
                            detect_mode = (detect_mode + 1) % DETECT_MODES.len();
                            let config = match detect_mode {
                                1 => AnalysisConfig { enable_bpm_fusion: true, ..AnalysisConfig::default() },
                                2 => AnalysisConfig { force_legacy_bpm: true, ..AnalysisConfig::default() },
                                _ => AnalysisConfig::default(),
                            };
                            let mono_bg = Arc::clone(mono);
                            let offset_snap = offset_ms;
                            let (tx, rx) = mpsc::channel::<(String, f32, i64)>();
                            thread::spawn(move || {
                                if let Ok(result) = analyze_audio(&mono_bg, sample_rate, config) {
                                    let _ = tx.send((hash, result.bpm.round(), offset_snap));
                                }
                            });
                            bpm_rx = rx;
                            analysis_hash = None;
                            tap_guided_rx = false;
                            tap_offset_pending = None;
                        }
                    }
                    Some(Action::JumpForward1)  => do_jump(&seek_handle, &player, bpm, total_duration,  1),
                    Some(Action::JumpBackward1) => do_jump(&seek_handle, &player, bpm, total_duration, -1),
                    Some(Action::JumpForward4)  => do_jump(&seek_handle, &player, bpm, total_duration,  4),
                    Some(Action::JumpBackward4) => do_jump(&seek_handle, &player, bpm, total_duration, -4),
                    Some(Action::JumpForward16) => do_jump(&seek_handle, &player, bpm, total_duration,  16),
                    Some(Action::JumpBackward16)=> do_jump(&seek_handle, &player, bpm, total_duration, -16),
                    Some(Action::JumpForward64) => do_jump(&seek_handle, &player, bpm, total_duration,  64),
                    Some(Action::JumpBackward64)=> do_jump(&seek_handle, &player, bpm, total_duration, -64),
                    Some(Action::TempoReset) => {
                        bpm = base_bpm;
                        player.set_speed(1.0);
                    }
                    Some(Action::BpmTap) => {
                        let now = Instant::now();
                        if let Some(last) = last_tap_wall {
                            if now.duration_since(last).as_secs_f64() > 2.0 {
                                tap_times.clear();
                                tap_offset_pending = None;
                            }
                        }
                        tap_times.push(smooth_display_samp / sample_rate as f64);
                        last_tap_wall = Some(now);
                        if tap_times.len() >= 8 {
                            let (tapped_bpm, tapped_offset) = compute_tap_bpm_offset(&tap_times);
                            // Preserve any f/v speed ratio across the base_bpm correction.
                            // Without this, bpm stays at the old detected value (e.g. 117)
                            // while base_bpm changes (e.g. to 120), making playback 97.5% speed
                            // and causing ticks to drift ahead of the audio.
                            let speed_ratio = bpm / base_bpm;
                            base_bpm = tapped_bpm;
                            bpm = (base_bpm * speed_ratio).clamp(40.0, 240.0);
                            offset_ms = tapped_offset;
                            player.set_speed(bpm / base_bpm);
                            // Analysis is launched when the session ends (2s after last tap).
                        }
                    }
                    Some(Action::NudgeBackward) | Some(Action::NudgeForward) | Some(Action::NudgeModeToggle) => {}
                    None => {}
                    }
                } // end if Press
            }
            _ => {}
            }
        }

        if player.empty() && !player.is_paused() {
            player.pause();
            let total_mono_samps = seek_handle.samples.len() / seek_handle.channels as usize;
            smooth_display_samp = total_mono_samps as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Beat marker helpers
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// Braille rendering helpers
// ---------------------------------------------------------------------------

/// Pre-rendered braille buffer wider than the visible area, enabling smooth scrolling.
/// The UI thread pans a viewport through this stable buffer rather than requesting
/// a full recompute every time the playhead advances by one column.
struct BrailleBuffer {
    grid:            Vec<Vec<u8>>, // rows × buf_cols braille bytes
    buf_cols:        usize,        // total buffer width (= 3 × screen_cols)
    anchor_sample:   usize,        // mono-sample index at the buffer centre
    samples_per_col: usize,        // mono samples represented by each buffer column
}

impl BrailleBuffer {
    fn empty() -> Self {
        Self { grid: Vec::new(), buf_cols: 0, anchor_sample: 0, samples_per_col: 1 }
    }
}


/// Pre-render a waveform peak grid into a rows×cols array of Braille dot-pattern bytes.
///
/// Each byte encodes which dots are lit in the corresponding Braille cell (U+2800 + byte).
/// Both the left and right dot columns are set for every lit dot row, so each column of the
/// waveform appears as a solid-width vertical bar.
///
/// `peaks` — one (min, max) pair per column, values in [-1, 1].
/// Mapping: y = +1 → top dot row 0; y = −1 → bottom dot row (rows×4 − 1).
/// Combine two adjacent braille bytes into a half-column-shifted result.
#[derive(Clone, Copy, PartialEq)]
enum NudgeMode { Jump, Warp }

// ---------------------------------------------------------------------------
// Keyboard mapping
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Action {
    PlayPause, Quit,
    JumpForward1, JumpForward4, JumpForward16, JumpForward64,
    JumpBackward1, JumpBackward4, JumpBackward16, JumpBackward64,
    NudgeBackward, NudgeForward, NudgeModeToggle,
    OffsetIncrease, OffsetDecrease,
    ZoomIn, ZoomOut,
    HeightIncrease, HeightDecrease,
    VolumeUp, VolumeDown,
    BpmHalve, BpmDouble, BpmIncrease, BpmDecrease, BaseBpmIncrease, BaseBpmDecrease,
    BpmRedetect, BpmTap,
    PaletteCycle, OpenBrowser, Help, WaveformStyle, TempoReset,
    CalibrationToggle,
}

static ACTION_NAMES: &[(&str, Action)] = &[
    ("play_pause",       Action::PlayPause),
    ("quit",             Action::Quit),
    ("jump_forward_1",   Action::JumpForward1),
    ("jump_forward_4",   Action::JumpForward4),
    ("jump_forward_16",  Action::JumpForward16),
    ("jump_forward_64",  Action::JumpForward64),
    ("jump_backward_1",  Action::JumpBackward1),
    ("jump_backward_4",  Action::JumpBackward4),
    ("jump_backward_16", Action::JumpBackward16),
    ("jump_backward_64", Action::JumpBackward64),
    ("nudge_backward",   Action::NudgeBackward),
    ("nudge_forward",    Action::NudgeForward),
    ("nudge_mode_toggle",Action::NudgeModeToggle),
    ("offset_increase",  Action::OffsetIncrease),
    ("offset_decrease",  Action::OffsetDecrease),
    ("zoom_in",          Action::ZoomIn),
    ("zoom_out",         Action::ZoomOut),
    ("height_increase",  Action::HeightIncrease),
    ("height_decrease",  Action::HeightDecrease),
    ("volume_up",        Action::VolumeUp),
    ("volume_down",      Action::VolumeDown),
    ("bpm_halve",        Action::BpmHalve),
    ("bpm_double",       Action::BpmDouble),
    ("bpm_increase",      Action::BpmIncrease),
    ("bpm_decrease",      Action::BpmDecrease),
    ("base_bpm_increase", Action::BaseBpmIncrease),
    ("base_bpm_decrease", Action::BaseBpmDecrease),
    ("bpm_redetect",     Action::BpmRedetect),
    ("bpm_tap",          Action::BpmTap),
    ("palette_cycle",    Action::PaletteCycle),
    ("open_browser",     Action::OpenBrowser),
    ("help",             Action::Help),
    ("waveform_style",       Action::WaveformStyle),
    ("tempo_reset",          Action::TempoReset),
    ("calibration_toggle",   Action::CalibrationToggle),
];

#[derive(Hash, Eq, PartialEq)]
enum KeyBinding {
    Key(KeyCode),
    SpaceChord(KeyCode),
}

fn parse_key(s: &str) -> Option<KeyBinding> {
    if let Some(rest) = s.strip_prefix("space+") {
        return parse_bare_key(rest).map(KeyBinding::SpaceChord);
    }
    parse_bare_key(s).map(KeyBinding::Key)
}

fn parse_bare_key(s: &str) -> Option<KeyCode> {
    match s {
        "space"     => Some(KeyCode::Char(' ')),
        "left"      => Some(KeyCode::Left),
        "right"     => Some(KeyCode::Right),
        "up"        => Some(KeyCode::Up),
        "down"      => Some(KeyCode::Down),
        "enter"     => Some(KeyCode::Enter),
        "backspace" => Some(KeyCode::Backspace),
        "esc"       => Some(KeyCode::Esc),
        s if s.chars().count() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        other => {
            eprintln!("tj: unknown key {:?} in config — binding skipped", other);
            None
        }
    }
}

const DEFAULT_CONFIG: &str = include_str!("../resources/config.toml");

struct DisplayConfig {
    playhead_position: u8,      // 0–100, clamped
    warning_threshold_secs: f32, // seconds before end to activate warning flash
}

impl Default for DisplayConfig {
    fn default() -> Self { Self { playhead_position: 20, warning_threshold_secs: 30.0 } }
}

/// Finds or creates the config file and returns its text.
fn resolve_config() -> String {
    // Check next to the binary first, then ~/.config/tj/config.toml, then auto-create.
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.toml")))
        .filter(|p| p.exists());
    if let Some(path) = adjacent {
        return std::fs::read_to_string(&path).unwrap_or_default();
    }
    let user_path = match home_dir() {
        Some(h) => h.join(".config/tj/config.toml"),
        None => return DEFAULT_CONFIG.to_string(),
    };
    if user_path.exists() {
        std::fs::read_to_string(&user_path).unwrap_or_default()
    } else {
        // Auto-create from embedded default.
        if let Some(dir) = user_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::write(&user_path, DEFAULT_CONFIG).is_ok() {
            eprintln!("tj: created default config at {}", user_path.display());
        }
        DEFAULT_CONFIG.to_string()
    }
}

fn load_config() -> (std::collections::HashMap<KeyBinding, Action>, DisplayConfig) {
    let text = resolve_config();
    let mut map = std::collections::HashMap::new();
    let keymap = parse_keymap(&text, &mut map);
    let display = parse_display_config(&text);
    (keymap, display)
}

fn parse_display_config(text: &str) -> DisplayConfig {
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(_) => return DisplayConfig::default(),
    };
    let display = parsed.get("display");
    let pos = display
        .and_then(|v| v.get("playhead_position"))
        .and_then(|v| v.as_integer())
        .unwrap_or(20)
        .clamp(0, 100) as u8;
    let warning_threshold_secs = display
        .and_then(|v| v.get("warning_threshold_secs"))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(30.0)
        .max(0.0) as f32;
    DisplayConfig { playhead_position: pos, warning_threshold_secs }
}

fn parse_keymap(text: &str, map: &mut std::collections::HashMap<KeyBinding, Action>)
    -> std::collections::HashMap<KeyBinding, Action>
{
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(e) => { eprintln!("tj: failed to parse config: {e}"); return std::mem::take(map); }
    };
    let keys = match parsed.get("keys").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return std::mem::take(map),
    };
    for (name, val) in keys {
        let action = match ACTION_NAMES.iter().find(|(n, _)| *n == name.as_str()) {
            Some((_, a)) => *a,
            None => { eprintln!("tj: unknown function {name:?} in config — skipped"); continue; }
        };
        let key_strs: Vec<&str> = if let Some(s) = val.as_str() {
            vec![s]
        } else if let Some(arr) = val.as_array() {
            arr.iter().filter_map(|v| v.as_str()).collect()
        } else {
            eprintln!("tj: key value for {name:?} must be a string or array of strings");
            continue;
        };
        for key_str in key_strs {
            if let Some(binding) = parse_key(key_str) {
                map.insert(binding, action);
            }
        }
    }
    std::mem::take(map)
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

/// Compute BPM and phase offset from a list of tap times (track position in seconds).
/// BPM = 60 / median inter-tap interval.
/// Offset = mean residual anchored to the first tap, avoiding phase drift from imprecise period.
fn compute_tap_bpm_offset(tap_times: &[f64]) -> (f32, i64) {
    let intervals: Vec<f64> = tap_times.windows(2).map(|w| w[1] - w[0]).collect();
    let mut sorted = intervals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    let beat_period = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };
    if beat_period <= 0.0 { return (120.0, 0); }
    let bpm = (60.0 / beat_period) as f32;
    // Anchor residuals to the first tap so deltas are small.
    // Computing t % beat_period on large absolute positions causes phase drift when
    // beat_period is even slightly imprecise — error accumulates with distance from zero.
    let t0 = tap_times[0];
    let mean_residual = tap_times.iter()
        .map(|&t| { let d = t - t0; d - (d / beat_period).round() * beat_period })
        .sum::<f64>() / tap_times.len() as f64;
    let offset_secs = (t0 + mean_residual).rem_euclid(beat_period);
    let offset_ms = (offset_secs * 1000.0).round() as i64;
    (bpm.clamp(40.0, 240.0), offset_ms)
}

/// Play a one-shot scrub snippet from the interleaved sample buffer at the given mono position.
/// Injects directly into the mixer so it plays independently of the paused main player.
fn scrub_audio(
    mixer: &rodio::mixer::Mixer,
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    mono_pos: usize,
    mono_len: usize,
) {
    use rodio::buffer::SamplesBuffer;
    use std::num::NonZero;
    let start = (mono_pos * channels as usize).min(samples.len());
    let end = ((mono_pos + mono_len) * channels as usize).min(samples.len());
    if start >= end { return; }
    let snippet: Vec<f32> = samples[start..end].to_vec();
    let src = SamplesBuffer::new(
        NonZero::new(channels).unwrap(),
        NonZero::new(sample_rate).unwrap(),
        snippet,
    );
    mixer.add(src);
}

/// Synthesise a short 1 kHz click tone and inject it into the mixer.
fn play_click_tone(mixer: &rodio::mixer::Mixer, sample_rate: u32) {
    use rodio::buffer::SamplesBuffer;
    use std::num::NonZero;
    let total = (sample_rate as usize * 20 / 1000).max(1); // 20 ms
    let attack = (sample_rate as usize * 2 / 1000).max(1); // 2 ms
    let samples: Vec<f32> = (0..total)
        .map(|i| {
            let envelope = if i < attack {
                i as f32 / attack as f32
            } else {
                1.0 - (i - attack) as f32 / (total - attack).max(1) as f32
            };
            let phase = 2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sample_rate as f32;
            phase.sin() * envelope * 0.4
        })
        .collect();
    let src = SamplesBuffer::new(
        NonZero::new(1u16).unwrap(),
        NonZero::new(sample_rate).unwrap(),
        samples,
    );
    mixer.add(src);
}

/// Beat-jump helper. Positive `beats` = forward, negative = backward.
fn do_jump(seek_handle: &SeekHandle, player: &rodio::Player, bpm: f32, total_duration: std::time::Duration, beats: i32) {
    let jump = beats.unsigned_abs() as f64 * 60.0 / bpm as f64;
    if beats < 0 {
        let target = (seek_handle.current_pos().as_secs_f64() - jump).max(0.0);
        if player.is_paused() { seek_handle.seek_direct(target); } else { seek_handle.seek_to(target); }
    } else {
        let target = seek_handle.current_pos().as_secs_f64() + jump;
        if target < total_duration.as_secs_f64() {
            if player.is_paused() { seek_handle.seek_direct(target); } else { seek_handle.seek_to(target); }
        }
    }
}

/// Takes the right dot-column of `a` (bits 3,4,5,7) as the new left column (bits 0,1,2,6)
/// and the left dot-column of `b` (bits 0,1,2,6) as the new right column (bits 3,4,5,7).
fn shift_braille_half(a: u8, b: u8) -> u8 {
    let left  = ((a >> 3) & 0x07) | ((a >> 1) & 0x40);
    let right = ((b & 0x07) << 3) | ((b & 0x40) << 1);
    left | right
}

fn render_braille(peaks: &[(f32, f32)], rows: usize, cols: usize, outline: bool) -> Vec<Vec<u8>> {
    // Bit mask for left+right dots at each of the 4 dot-rows within a Braille cell.
    // Layout: dot1(bit0)/dot4(bit3), dot2(bit1)/dot5(bit4), dot3(bit2)/dot6(bit5), dot7(bit6)/dot8(bit7)
    const DOT_BITS: [u8; 4] = [0x09, 0x12, 0x24, 0xC0];

    let mut grid = vec![vec![0u8; cols]; rows];
    if rows == 0 || cols == 0 {
        return grid;
    }
    let total_dots = rows * 4;

    let mut set_dot = |c: usize, d: usize| {
        let br = d / 4;
        let dr = d % 4;
        if br < rows {
            grid[br][c] |= DOT_BITS[dr];
        }
    };

    let mut prev_top: Option<usize> = None;
    let mut prev_bot: Option<usize> = None;

    for (c, &(min_val, max_val)) in peaks.iter().take(cols).enumerate() {
        let clamped_max = max_val.min(1.0);
        let clamped_min = min_val.max(-1.0);
        if clamped_min > clamped_max {
            prev_top = None;
            prev_bot = None;
            continue;
        }
        // Map y ∈ [-1, 1] → dot row ∈ [0, total_dots); y=1 is top (row 0).
        let top_dot = ((1.0 - clamped_max) / 2.0 * total_dots as f32) as usize;
        let bot_dot = (((1.0 - clamped_min) / 2.0 * total_dots as f32) as usize)
            .min(total_dots - 1);
        if outline {
            // Bridge vertical gap to previous column so the outline is continuous.
            let top_from = prev_top.map(|p| p.min(top_dot)).unwrap_or(top_dot);
            let top_to   = prev_top.map(|p| p.max(top_dot)).unwrap_or(top_dot);
            for d in top_from..=top_to { set_dot(c, d); }
            if bot_dot != top_dot {
                let bot_from = prev_bot.map(|p| p.min(bot_dot)).unwrap_or(bot_dot);
                let bot_to   = prev_bot.map(|p| p.max(bot_dot)).unwrap_or(bot_dot);
                for d in bot_from..=bot_to { set_dot(c, d); }
            }
            prev_top = Some(top_dot);
            prev_bot = Some(bot_dot);
        } else {
            for d in top_dot..=bot_dot { set_dot(c, d); }
        }
    }
    grid
}

/// Return the column indices of beat lines within the detail view window.
///
/// Replaces `draw_beat_lines` — callers colour these columns instead of drawing Canvas lines.

/// Return the column indices of bar-tick lines within the overview, and the bars-per-tick interval.
///
/// Starts at 4 bars and doubles until all adjacent ticks are at least 2 columns apart
/// (leaving at least 1 blank character gap between every pair of markers).
fn bar_tick_cols(bpm: f64, offset_ms: i64, total_secs: f64, cols: usize) -> (Vec<usize>, u32) {
    if bpm <= 0.0 || total_secs <= 0.0 || cols == 0 {
        return (Vec::new(), 4);
    }
    let beat_secs = 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let mut bars: u32 = 4;
    loop {
        let bar_period = bars as f64 * 4.0 * beat_secs; // bars × 4 beats/bar × secs/beat
        let n_start = (-offset_secs / bar_period).ceil() as i64;
        let mut result = Vec::new();
        let mut t = offset_secs + n_start as f64 * bar_period;
        while t <= total_secs {
            let col = ((t / total_secs) * cols as f64).round() as usize;
            if col < cols {
                result.push(col);
            }
            t += bar_period;
        }
        let min_gap = result.windows(2)
            .map(|w| w[1].saturating_sub(w[0]))
            .min()
            .unwrap_or(usize::MAX);
        if min_gap >= 2 || bars >= 512 {
            return (result, bars);
        }
        bars *= 2;
    }
}

// ---------------------------------------------------------------------------
// Waveform data
// ---------------------------------------------------------------------------

struct WaveformData {
    /// Full-track peak envelope at OVERVIEW_RESOLUTION buckets.
    peaks: Vec<(f32, f32)>,
    /// Per-bucket bass ratio in [0,1]: 1.0 = bass-heavy, 0.0 = treble-heavy.
    bass_ratio: Vec<f32>,
    /// Raw mono samples for detail view rendering.
    mono: Arc<Vec<f32>>,
}

impl WaveformData {
    fn compute(mono: Arc<Vec<f32>>, sample_rate: u32) -> Self {
        let n = mono.len();
        let chunk_size = (n / OVERVIEW_RESOLUTION).max(1);
        // k: spectral rate at crossover (250 Hz); bass_ratio = 0.5 at this frequency.
        let k = (2.0 * std::f32::consts::TAU * 250.0 / sample_rate as f32).max(1e-6);
        let (peaks, bass_ratio) = mono
            .chunks(chunk_size)
            .map(|chunk| {
                let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let total_energy: f32 = chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32;
                let diff_energy: f32 = chunk.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>()
                    / (chunk.len() as f32 - 1.0).max(1.0);
                let bass = (1.0 - (diff_energy / (total_energy + 1e-10)).sqrt() / k).clamp(0.0, 1.0);
                ((min.max(-1.0), max.min(1.0)), bass)
            })
            .unzip();
        Self { peaks, bass_ratio, mono }
    }
}

// ---------------------------------------------------------------------------
// Custom rodio Source + SeekHandle
// ---------------------------------------------------------------------------

struct TrackingSource {
    samples: Arc<Vec<f32>>,
    position: Arc<AtomicUsize>,
    /// Fade state: negative = fading out (counting toward 0), positive = fading in (counting down).
    fade_remaining: Arc<AtomicI64>,
    /// Length of the current fade in samples (FADE_SAMPLES or MICRO_FADE_SAMPLES).
    fade_len: Arc<AtomicI64>,
    /// Pending seek target sample index; usize::MAX means no seek pending.
    pending_target: Arc<AtomicUsize>,
    sample_rate: u32,
    channels: u16,
}

impl TrackingSource {
    fn new(
        samples: Arc<Vec<f32>>,
        position: Arc<AtomicUsize>,
        fade_remaining: Arc<AtomicI64>,
        fade_len: Arc<AtomicI64>,
        pending_target: Arc<AtomicUsize>,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        Self { samples, position, fade_remaining, fade_len, pending_target, sample_rate, channels }
    }
}

impl Iterator for TrackingSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let fade = self.fade_remaining.load(Ordering::Relaxed);

        if fade < 0 {
            // Fading out: read current position, apply descending envelope.
            let fl = self.fade_len.load(Ordering::Relaxed);
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            let raw = self.samples.get(pos).copied().unwrap_or(0.0);
            let t = (-fade) as f32 / fl as f32;
            let new_fade = self.fade_remaining.fetch_add(1, Ordering::Relaxed) + 1;
            if new_fade == 0 {
                // Fade-out complete — apply pending seek then start fade-in.
                let target = self.pending_target.swap(usize::MAX, Ordering::SeqCst);
                if target != usize::MAX {
                    self.position.store(target, Ordering::SeqCst);
                }
                self.fade_remaining.store(fl, Ordering::Relaxed);
            }
            Some(raw * t)
        } else if fade > 0 {
            // Fading in: read new position, apply ascending envelope.
            let fl = self.fade_len.load(Ordering::Relaxed);
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            let raw = self.samples.get(pos).copied().unwrap_or(0.0);
            let t = (fl - fade) as f32 / fl as f32;
            self.fade_remaining.fetch_sub(1, Ordering::Relaxed);
            Some(raw * t)
        } else {
            // Normal playback.
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            self.samples.get(pos).copied()
        }
    }
}

impl Source for TrackingSource {
    fn current_span_len(&self) -> Option<usize> {
        // Return a short span so UniformSourceIterator re-checks sample_rate() frequently,
        // enabling Player::set_speed() changes to take effect within ~100ms.
        Some(self.sample_rate as usize / 10 * self.channels as usize)
    }
    fn channels(&self) -> NonZero<u16> {
        NonZero::new(self.channels).unwrap_or(NonZero::new(2).unwrap())
    }
    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(self.sample_rate).unwrap_or(NonZero::new(44100).unwrap())
    }
    fn total_duration(&self) -> Option<Duration> { None }
}

/// Shared handle for querying playback position and seeking without interrupting the audio thread.
struct SeekHandle {
    samples: Arc<Vec<f32>>,
    position: Arc<AtomicUsize>,
    fade_remaining: Arc<AtomicI64>,
    fade_len: Arc<AtomicI64>,
    pending_target: Arc<AtomicUsize>,
    sample_rate: u32,
    channels: u16,
}

impl SeekHandle {
    /// Current playback position derived from the atomic sample counter.
    fn current_pos(&self) -> Duration {
        let pos = self.position.load(Ordering::Relaxed);
        Duration::from_secs_f64(pos as f64 / (self.sample_rate as f64 * self.channels as f64))
    }

    /// Seek to `target_secs`. Triggers a fade-out on the audio thread, which then
    /// atomically jumps to the target and fades back in — no gap, no click.
    fn seek_to(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as i64;
        let window = self.sample_rate as i64 / 100; // 10ms in frames

        let search_start = (target_frame - window).max(0) as usize;
        let search_end = (target_frame + window).min(total_frames as i64) as usize;

        // Find the quietest frame near the target — minimises the fade-in transient.
        let best_frame = (search_start..=search_end)
            .min_by_key(|&f| {
                let base = f * frame_len;
                let amp: f32 = (0..frame_len)
                    .map(|c| self.samples.get(base + c).copied().unwrap_or(0.0).abs())
                    .sum();
                (amp * 1_000_000.0) as u64
            })
            .unwrap_or(target_frame.max(0) as usize);

        let target_sample = (best_frame * frame_len).min(self.samples.len());

        // Store the target, then trigger fade-out. The audio thread applies the seek
        // when the fade-out completes and then fades back in.
        self.fade_len.store(FADE_SAMPLES, Ordering::SeqCst);
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-FADE_SAMPLES, Ordering::SeqCst);
    }

    /// Seek to `target_secs` with a short ~0.2ms fade — for micro-jumps where a full
    /// 6ms fade would be longer than the jump itself.
    fn seek_micro_fade(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as usize;
        let target_sample = (target_frame * frame_len).min(self.samples.len());
        self.fade_len.store(MICRO_FADE_SAMPLES, Ordering::SeqCst);
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-MICRO_FADE_SAMPLES, Ordering::SeqCst);
        let _ = total_frames; // used implicitly via clamp above
    }

    /// Seek to `target_secs` directly, without a fade. Used when paused — the audio
    /// thread is not calling next(), so the fade-based seek would never execute.

    fn seek_direct(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as i64;
        let window = self.sample_rate as i64 / 100;

        let search_start = (target_frame - window).max(0) as usize;
        let search_end = (target_frame + window).min(total_frames as i64) as usize;

        let best_frame = (search_start..=search_end)
            .min_by_key(|&f| {
                let base = f * frame_len;
                let amp: f32 = (0..frame_len)
                    .map(|c| self.samples.get(base + c).copied().unwrap_or(0.0).abs())
                    .sum();
                (amp * 1_000_000.0) as u64
            })
            .unwrap_or(target_frame.max(0) as usize);

        let target_sample = (best_frame * frame_len).min(self.samples.len());

        // Write position directly and clear any in-progress fade.
        self.pending_target.store(usize::MAX, Ordering::SeqCst);
        self.fade_remaining.store(0, Ordering::SeqCst);
        self.position.store(target_sample, Ordering::SeqCst);
    }

    /// Move to `target_secs` exactly, without a quiet-frame search or fade.
    /// Used for paused nudge where no click can occur.
    fn set_position(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as usize;
        let target_sample = target_frame.min(total_frames) * frame_len;
        self.pending_target.store(usize::MAX, Ordering::SeqCst);
        self.fade_remaining.store(0, Ordering::SeqCst);
        self.position.store(target_sample, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Audio decode
// ---------------------------------------------------------------------------

/// Decode an audio file. Returns (mono_f32, interleaved_f32, sample_rate, channels).
/// Updates `decoded_samples` and `estimated_total` atomics as decode progresses.
fn decode_audio(
    path: &str,
    decoded_samples: Arc<AtomicUsize>,
    estimated_total: Arc<AtomicUsize>,
) -> Result<(Vec<f32>, Vec<f32>, u32, u16), Box<dyn std::error::Error>> {
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("no audio track found")?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.ok_or("track has no sample rate")?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

    if let Some(n_frames) = track.codec_params.n_frames {
        estimated_total.store((n_frames as usize).saturating_mul(channels as usize), Ordering::Relaxed);
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != track_id { continue; }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let buf = sample_buf.get_or_insert_with(|| {
                    SampleBuffer::new(decoded.capacity() as u64, *decoded.spec())
                });
                buf.copy_interleaved_ref(decoded);
                interleaved.extend_from_slice(buf.samples());
                decoded_samples.store(interleaved.len(), Ordering::Relaxed);
            }
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        }
    }

    let mono: Vec<f32> = if channels > 1 {
        interleaved
            .chunks_exact(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        interleaved.clone()
    };

    Ok((mono, interleaved, sample_rate, channels))
}

// ---------------------------------------------------------------------------
// BPM cache
// ---------------------------------------------------------------------------

fn hash_mono(samples: &[f32]) -> String {
    let mut hasher = blake3::Hasher::new();
    for s in samples {
        hasher.update(&s.to_le_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn cache_path() -> std::path::PathBuf {
    let base = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join(".local/share/tj/cache.json")
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    bpm: f32,
    offset_ms: i64,
    /// Filename at time of first detection — informational only, not used as key.
    name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct CacheFile {
    #[serde(default)]
    last_browser_path: Option<String>,
    #[serde(default)]
    audio_latency_ms: i64,
    #[serde(default)]
    entries: std::collections::HashMap<String, CacheEntry>,
}

struct Cache {
    path: std::path::PathBuf,
    last_browser_path: Option<std::path::PathBuf>,
    audio_latency_ms: i64,
    entries: std::collections::HashMap<String, CacheEntry>,
}

impl Cache {
    fn load(path: std::path::PathBuf) -> Self {
        let file: CacheFile = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| {
                // Try new wrapped format first; fall back to legacy flat HashMap.
                serde_json::from_str::<CacheFile>(&text).ok().or_else(|| {
                    serde_json::from_str::<std::collections::HashMap<String, CacheEntry>>(&text)
                        .ok()
                        .map(|entries| CacheFile { entries, ..Default::default() })
                })
            })
            .unwrap_or_default();
        Self {
            path,
            last_browser_path: file.last_browser_path.map(std::path::PathBuf::from),
            audio_latency_ms: file.audio_latency_ms,
            entries: file.entries,
        }
    }

    fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.entries.get(hash)
    }

    fn set(&mut self, hash: String, entry: CacheEntry) {
        self.entries.insert(hash, entry);
    }

    fn last_browser_path(&self) -> Option<&std::path::Path> {
        self.last_browser_path.as_deref()
    }

    fn set_last_browser_path(&mut self, p: &std::path::Path) {
        self.last_browser_path = Some(p.to_path_buf());
    }

    fn get_latency(&self) -> i64 {
        self.audio_latency_ms
    }

    fn set_latency(&mut self, ms: i64) {
        self.audio_latency_ms = ms;
    }

    fn entries_snapshot(&self) -> std::collections::HashMap<String, CacheEntry> {
        self.entries.clone()
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("tmp");
        let file = CacheFile {
            last_browser_path: self.last_browser_path
                .as_ref()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            audio_latency_ms: self.audio_latency_ms,
            entries: self.entries.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&file) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BPM detection
// ---------------------------------------------------------------------------

fn detect_bpm(samples: &[f32], sample_rate: u32) -> Result<f32, Box<dyn std::error::Error>> {
    let result = analyze_audio(samples, sample_rate, AnalysisConfig::default())
        .map_err(|e| format!("stratum-dsp: {e:?}"))?;
    Ok(result.bpm)
}

// ---------------------------------------------------------------------------
// File browser
// ---------------------------------------------------------------------------

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "wav", "aac", "opus", "m4a"];

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Debug, PartialEq, Clone)]
enum EntryKind {
    Dir,
    Audio,
    Other,
}

struct BrowserEntry {
    name: String,
    path: std::path::PathBuf,
    kind: EntryKind,
}

struct BrowserState {
    cwd: std::path::PathBuf,
    entries: Vec<BrowserEntry>,
    cursor: usize,
}

impl BrowserState {
    fn new(dir: std::path::PathBuf) -> io::Result<Self> {
        let mut entries = Vec::new();

        if dir.parent().is_some() {
            entries.push(BrowserEntry {
                name: "..".to_string(),
                path: dir.parent().unwrap().to_path_buf(),
                kind: EntryKind::Dir,
            });
        }

        let mut raw: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
        raw.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

        for entry in raw {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let kind = if path.is_dir() {
                EntryKind::Dir
            } else if is_audio(&path) {
                EntryKind::Audio
            } else {
                EntryKind::Other
            };
            entries.push(BrowserEntry { name, path, kind });
        }

        let cursor = entries
            .iter()
            .position(|e| Self::is_selectable(&e.kind))
            .unwrap_or(0);

        Ok(Self { cwd: dir, entries, cursor })
    }

    fn is_selectable(kind: &EntryKind) -> bool {
        matches!(kind, EntryKind::Dir | EntryKind::Audio)
    }

    fn move_down(&mut self) {
        let next = (self.cursor + 1..self.entries.len())
            .find(|&i| Self::is_selectable(&self.entries[i].kind));
        if let Some(i) = next {
            self.cursor = i;
        }
    }

    fn move_up(&mut self) {
        let prev = (0..self.cursor)
            .rev()
            .find(|&i| Self::is_selectable(&self.entries[i].kind));
        if let Some(i) = prev {
            self.cursor = i;
        }
    }
}

enum BrowserResult {
    Selected(std::path::PathBuf),
    ReturnToPlayer,
    Quit,
}

fn run_browser(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: std::path::PathBuf,
) -> io::Result<(BrowserResult, std::path::PathBuf)> {
    let mut state = BrowserState::new(start_dir)?;

    loop {
        let cursor = state.cursor;
        let cwd_display = state.cwd.display().to_string();

        let items: Vec<ListItem> = state
            .entries
            .iter()
            .map(|e| {
                let (label, color) = match e.kind {
                    EntryKind::Dir => {
                        let label = format!("{}/", e.name);
                        (label, Color::Yellow)
                    }
                    EntryKind::Audio => (e.name.clone(), Color::Green),
                    EntryKind::Other => (e.name.clone(), Color::DarkGray),
                };
                ListItem::new(label).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" tj — {} ", cwd_display))
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut list_state = ListState::default().with_selected(Some(cursor));

        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(area);

            frame.render_stateful_widget(list, chunks[0], &mut list_state);

            frame.render_widget(
                Paragraph::new("↑/↓: navigate   Enter: open   ←/Bksp: up dir   Esc: back to player   q: quit")
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[1],
            );
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok((BrowserResult::Quit, state.cwd));
                }
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Up => state.move_up(),
                    KeyCode::Down => state.move_down(),
                    KeyCode::Enter => {
                        if let Some(entry) = state.entries.get(state.cursor) {
                            match entry.kind {
                                EntryKind::Dir => {
                                    let path = entry.path.clone();
                                    state = BrowserState::new(path)?;
                                }
                                EntryKind::Audio => {
                                    let cwd = state.cwd.clone();
                                    return Ok((BrowserResult::Selected(entry.path.clone()), cwd));
                                }
                                EntryKind::Other => {}
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Left => {
                        if let Some(parent) = state.cwd.parent().map(|p| p.to_path_buf()) {
                            state = BrowserState::new(parent)?;
                        }
                    }
                    KeyCode::Char('q') => return Ok((BrowserResult::Quit, state.cwd)),
                    KeyCode::Esc => return Ok((BrowserResult::ReturnToPlayer, state.cwd)),
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_state(kinds: &[EntryKind]) -> BrowserState {
        BrowserState {
            cwd: PathBuf::from("/test"),
            entries: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| BrowserEntry {
                    name: format!("entry{i}"),
                    path: PathBuf::from(format!("/test/entry{i}")),
                    kind: k.clone(),
                })
                .collect(),
            cursor: 0,
        }
    }

    #[test]
    fn test_is_audio_known_extensions() {
        assert!(is_audio(&PathBuf::from("track.flac")));
        assert!(is_audio(&PathBuf::from("track.mp3")));
        assert!(is_audio(&PathBuf::from("track.ogg")));
        assert!(is_audio(&PathBuf::from("track.wav")));
    }

    #[test]
    fn test_is_audio_case_insensitive() {
        assert!(is_audio(&PathBuf::from("track.FLAC")));
        assert!(is_audio(&PathBuf::from("track.Mp3")));
    }

    #[test]
    fn test_is_audio_non_audio() {
        assert!(!is_audio(&PathBuf::from("readme.txt")));
        assert!(!is_audio(&PathBuf::from("noextension")));
        assert!(!is_audio(&PathBuf::from("image.png")));
    }

    #[test]
    fn test_dirs_and_audio_are_selectable() {
        assert!(BrowserState::is_selectable(&EntryKind::Dir));
        assert!(BrowserState::is_selectable(&EntryKind::Audio));
        assert!(!BrowserState::is_selectable(&EntryKind::Other));
    }

    #[test]
    fn test_cursor_down_skips_other() {
        // [Audio, Other, Audio] — down from 0 should land on 2
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 0;
        state.move_down();
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_cursor_up_skips_other() {
        // [Audio, Other, Audio] — up from 2 should land on 0
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 2;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_cursor_down_does_not_pass_end() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 1;
        state.move_down();
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_cursor_up_does_not_pass_start() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 0;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }
}
