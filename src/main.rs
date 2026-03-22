use std::io;
use color_eyre::Result as EyreResult;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicUsize, Ordering};
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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

use rodio::stream::DeviceSinkBuilder;
use rodio::{Player, Source};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

use serde::{Deserialize, Serialize};
use stratum_dsp::{analyze_audio, AnalysisConfig};

const OVERVIEW_RESOLUTION: usize = 4000;
const ZOOM_LEVELS: &[f32] = &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const DEFAULT_ZOOM_IDX: usize = 2; // 4 seconds
const FADE_SAMPLES: i64 = 256;       // ~5.8ms at 44100 Hz — fade-out then fade-in around each seek
/// Spectral colour palettes: (name, bass_rgb, treble_rgb).
const SPECTRAL_PALETTES: &[(&str, (u8,u8,u8), (u8,u8,u8))] = &[
    ("amber/cyan", (255, 140,   0), (  0, 200, 200)),
    ("soft",       (200, 130,  50), ( 50, 190, 200)),
    ("spectrum",   ( 80, 110, 220), (220, 200,  60)),
    ("green",      (120, 200,  60), ( 60, 200, 170)),
];

fn cleanup_terminal() {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
}

fn main() {
    color_eyre::install().expect("color_eyre initialisation should succeed at startup");
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

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            cleanup_terminal();
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();

    let initial_load: Option<PendingLoad> = if start.is_file() {
        Some(start_load(&start))
    } else {
        None
    };
    if let Err(e) = tui_loop(&mut terminal, initial_load, &mut cache, &mut browser_dir, &mixer) {
        cleanup_terminal();
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }

    cleanup_terminal();
}

struct PendingLoad {
    filename: String,
    path:     PathBuf,
    rx:       mpsc::Receiver<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>,
    decoded:  Arc<AtomicUsize>,
    total:    Arc<AtomicUsize>,
}

fn start_load(path: &Path) -> PendingLoad {
    let path_str = path.to_string_lossy().to_string();
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path_str)
        .to_string();
    let decoded = Arc::new(AtomicUsize::new(0));
    let total   = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>();
    {
        let decoded_for_thread = Arc::clone(&decoded);
        let total_for_thread   = Arc::clone(&total);
        thread::spawn(move || {
            let _ = tx.send(decode_audio(&path_str, decoded_for_thread, total_for_thread).map_err(|e| e.to_string()));
        });
    }
    PendingLoad { filename, path: path.to_path_buf(), rx, decoded, total }
}

fn build_deck(
    path:        &Path,
    filename:    String,
    mono:        Vec<f32>,
    stereo:      Vec<f32>,
    sample_rate: u32,
    channels:    u16,
    mixer:       &rodio::mixer::Mixer,
    cache:       &Cache,
) -> Deck {
    let track_name     = read_track_name(&path.to_string_lossy());
    let total_duration = Duration::from_secs(mono.len() as u64 / sample_rate as u64);
    let mono           = Arc::new(mono);
    let waveform       = Arc::new(WaveformData::compute(Arc::clone(&mono), sample_rate));

    let samples        = Arc::new(stereo);
    let position       = Arc::new(AtomicUsize::new(0));
    let fade_remaining = Arc::new(AtomicI64::new(0));
    let fade_len       = Arc::new(AtomicI64::new(FADE_SAMPLES));
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

    let filter_offset_shared = Arc::new(AtomicI32::new(0));
    let filter_state_reset   = Arc::new(AtomicBool::new(false));
    let player = Player::connect_new(mixer);
    player.append(FilterSource::new(
        TrackingSource::new(
            samples, position, fade_remaining, fade_len, pending_target, sample_rate, channels,
        ),
        Arc::clone(&filter_offset_shared),
        Arc::clone(&filter_state_reset),
    ));
    player.pause();

    let (bpm_tx, bpm_rx) = mpsc::channel::<(String, f32, i64, bool)>();
    {
        let mono_bg = Arc::clone(&mono);
        let entries = cache.entries_snapshot();
        thread::spawn(move || {
            let hash = hash_mono(&mono_bg);
            // is_fresh=false → applied immediately and marks bpm_established=true (confirmed).
            // is_fresh=true  → applied immediately only when bpm_established is false, leaves it false (unconfirmed).
            let (bpm, offset_ms, is_fresh) = if let Some(entry) = entries.get(&hash) {
                let snapped = (entry.offset_ms as f64 / 10.0).round() as i64 * 10;
                let period  = (60_000.0 / entry.bpm as f64 / 10.0).round() as i64 * 10;
                let snapped = snapped.rem_euclid(period);
                (entry.bpm, snapped, false)
            } else {
                // No cache entry: use 120 as a placeholder; leave bpm_established false so the UI
                // signals that the BPM has not been confirmed.
                (120.0f32, 0i64, true)
            };
            let _ = bpm_tx.send((hash, bpm, offset_ms, is_fresh));
        });
    }

    Deck::new(
        filename,
        track_name,
        total_duration,
        DeckAudio {
            player,
            seek_handle,
            mono,
            waveform,
            sample_rate,
            filter_offset_shared,
            filter_state_reset,
        },
        bpm_rx,
    )
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    initial_load: Option<PendingLoad>,
    cache: &mut Cache,
    browser_dir: &mut std::path::PathBuf,
    mixer: &rodio::mixer::Mixer,
) -> io::Result<()> {
    // Per-deck display values computed each frame from current deck state.
    struct DeckRenderState {
        display_samp:     f64,
        display_pos_samp: usize,
        analysing:        bool,
        spinner_active:   bool,
        beat_on:          bool,
        warning_active:   bool,
        warn_beat_on:     bool,
    }
    let (keymap, display_cfg, config_notice) = load_config();
    let mut global_notification: Option<Notification> = None;
    if let Some(msg) = config_notice {
        global_notification = Some(Notification {
            message: msg,
            style: NotificationStyle::Info,
            expires: Instant::now() + Duration::from_secs(8),
        });
    }
    let mut decks: [Option<Deck>; 2] = [None, None];
    let mut pending_loads: [Option<PendingLoad>; 2] = [initial_load, None];
    if pending_loads[0].is_none() && global_notification.is_none() {
        global_notification = Some(Notification {
            message: "No track loaded — press z to open the file browser".to_string(),
            style: NotificationStyle::Info,
            expires: Instant::now() + Duration::from_secs(60),
        });
    }
    const DET_MIN: u16 = 3;
    let mut audio_latency_ms: i64 = ((cache.get_latency() as f64 / 10.0).round() as i64 * 10).clamp(0, 250);
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let shared_renderer = SharedDetailRenderer::new(zoom_idx);
    let mut detail_height: usize = display_cfg.detail_height.max(DET_MIN as usize);
    let mut frame_count: usize = 0;
    let mut last_render = Instant::now();
    let mut help_open = false;
    let mut max_det_h: usize = usize::MAX;
    let mut space_held = false;
    // After a chord fires, suppress further Space-Press events until at least one frame
    // passes with no Space activity. Crossterm decodes Kitty key-repeats as Press events,
    // so without this guard the repeat stream re-arms space_held immediately after the
    // post-chord reset, leaving the modifier stuck until the repeats stop — which never
    // happens via a Release event (those also don't arrive in crossterm 0.29 + Kitty).
    let mut space_repeat_suppressed = false;
    let mut space_saw_event_this_frame = false;
    let mut pending_quit = false;
    // When Esc dismisses an overlay, suppress the next Quit action for a short window.
    // This absorbs the duplicate Esc event that Kitty injects after the first one.
    let mut suppress_quit_until: Option<Instant> = None;

    'tui: loop {
        frame_count += 1;

        // Clear the repeat-suppression latch once a full frame passes with no Space events,
        // indicating the key has been physically released.
        if space_repeat_suppressed && !space_saw_event_this_frame {
            space_repeat_suppressed = false;
        }
        space_saw_event_this_frame = false;

        // Frame timing — computed once and shared by both decks.
        let dc = shared_renderer.cols.load(Ordering::Relaxed);
        let zoom_secs = ZOOM_LEVELS[zoom_idx];
        let col_secs = if dc > 0 { zoom_secs as f64 / dc as f64 } else { 0.033 };

        // Frame budget: one half-column of scroll time, clamped to a sane range.
        // Sleep is deferred to the END of the loop so variable draw/write time is absorbed
        // automatically — the sleep shrinks to compensate for a slow terminal flush.
        let frame_dur = Duration::from_secs_f64(col_secs / 2.0)
            .max(Duration::from_millis(8))
            .min(Duration::from_millis(200));

        let frame_start = Instant::now();
        let elapsed = frame_start.duration_since(last_render).as_secs_f64()
            // Cap at 4 columns per frame. Must exceed the minimum frame_dur (8ms) at every zoom
            // level — a tighter cap causes systematic drift and periodic large-drift snapping.
            .min(col_secs * 4.0);
        last_render = frame_start;

        // Expire global notification.
        if global_notification.as_ref().map_or(false, |n| frame_start >= n.expires) {
            global_notification = None;
        }
        // Complete any pending loads.
        for slot in 0..2 {
            if pending_loads[slot].is_none() { continue; }
            let recv = pending_loads[slot].as_ref().unwrap().rx.try_recv();
            match recv {
                Ok(Ok((mono, stereo, sample_rate, channels))) => {
                    let pending = pending_loads[slot].take().unwrap();
                    let new_deck = build_deck(&pending.path, pending.filename, mono, stereo, sample_rate, channels, mixer, cache);
                    shared_renderer.set_deck(slot, Arc::clone(&new_deck.audio.waveform), new_deck.audio.seek_handle.channels, new_deck.audio.sample_rate);
                    decks[slot] = Some(new_deck);
                }
                Ok(Err(e)) => {
                    global_notification = Some(Notification {
                        message: format!("Load failed: {e}"),
                        style: NotificationStyle::Error,
                        expires: Instant::now() + Duration::from_secs(10),
                    });
                    pending_loads[slot] = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => { pending_loads[slot] = None; }
            }
        }

        // Service both decks: BPM results, position, metronome, tap timeout, spectrum.
        for slot in 0..2 {
            service_deck_frame(slot, &mut decks, col_secs, frame_dur, elapsed, mixer, &shared_renderer, cache, audio_latency_ms);
        }

        // Compute render state for both decks.
        let render: [Option<DeckRenderState>; 2] = std::array::from_fn(|slot| {
            let d = decks[slot].as_ref()?;
            // Latency correction only applies during playback — when paused there is
            // no buffer fill ahead, so the raw position is the heard position.
            let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
            let display_samp = (d.display.smooth_display_samp - latency_correction).max(0.0);
            let display_pos_samp = display_samp as usize;
            let pos_interleaved  = display_pos_samp * d.audio.seek_handle.channels as usize;
            if slot == 0 {
                shared_renderer.display_pos_a.store(pos_interleaved, Ordering::Relaxed);
            } else {
                shared_renderer.display_pos_b.store(pos_interleaved, Ordering::Relaxed);
            }
            let spinner_active = d.tempo.analysis_hash.is_none();
            let analysing      = spinner_active || !d.tempo.bpm_established;
            let beat_period    = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
            let flash_window   = beat_period.mul_f64(0.15);
            let smooth_pos_ns  = (display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                - d.tempo.offset_ms as i128 * 1_000_000;
            let phase          = smooth_pos_ns.rem_euclid(beat_period.as_nanos() as i128);
            let beat_on        = phase < flash_window.as_nanos() as i128;
            let audio_pos_samp = d.audio.seek_handle.position.load(Ordering::Relaxed)
                / d.audio.seek_handle.channels as usize;
            let pos_dur        = Duration::from_secs_f64(audio_pos_samp as f64 / d.audio.sample_rate as f64);
            let remaining      = d.total_duration.saturating_sub(pos_dur);
            let warning_active = !d.audio.player.is_paused()
                && remaining < Duration::from_secs_f32(display_cfg.warning_threshold_secs);
            let beat_index     = smooth_pos_ns.div_euclid(beat_period.as_nanos() as i128);
            let warn_beat_on   = warning_active && (beat_index % 2 == 0);
            Some(DeckRenderState { display_samp, display_pos_samp, analysing, spinner_active, beat_on, warning_active, warn_beat_on })
        });

        shared_renderer.zoom_at.store(zoom_idx, Ordering::Relaxed);
        let buf_a = Arc::clone(&*shared_renderer.shared_a.lock().unwrap());
        let buf_b = Arc::clone(&*shared_renderer.shared_b.lock().unwrap());
        let scrub_spc_a = buf_a.samples_per_col;
        let scrub_spc_b = buf_b.samples_per_col;

        // Take both decks out so the draw closure can mutate them.
        let mut d0 = decks[0].take();
        let mut d1 = decks[1].take();

        // Compute loading labels for slots that have a pending load but no deck.
        let loading_label: [Option<String>; 2] = std::array::from_fn(|slot| {
            let p = pending_loads[slot].as_ref()?;
            let done  = p.decoded.load(Ordering::Relaxed);
            let total = p.total.load(Ordering::Relaxed);
            let pct   = if total > 0 { format!(" {}%", (done * 100 / total).min(100)) } else { String::new() };
            Some(format!("Loading {}…{}", p.filename, pct))
        });

        terminal.draw(|frame| {
            let area = frame.area();
            let outer = Block::default()
                .title(format!(" tj {} ", env!("CARGO_PKG_VERSION")))
                .borders(Borders::ALL);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);

            // Compression order as the terminal shrinks:
            //   1. Detail waveforms compress evenly: detail_height → DET_MIN
            //   2. Overview waveforms compress evenly: OV_MAX → OV_MIN
            //   3. No further compression — elements fall off the bottom
            //
            // Row heights are pre-computed and sum exactly to inner.height so the
            // cassowary solver never receives an infeasible system and proportionally
            // shrinks things it shouldn't.
            const OV_MAX:  u16 = 3;
            const OV_MIN:  u16 = 2;
            let det_max = detail_height as u16;
            let ih = inner.height;
            let fixed = 6_u16; // global + detail-info + notif×2 + info×2

            // Cap detail_height to what the current terminal can actually display,
            // so HeightIncrease never outruns the screen.
            max_det_h = (ih.saturating_sub(fixed + OV_MIN * 2) / 2) as usize;

            // Compute a unified pool for each waveform type so both decks always
            // get the same height (no sequential-allocation asymmetry).
            // Phase 1: detail compresses; overviews stay at OV_MAX.
            // Phase 2: overviews compress; detail stays at DET_MIN.
            // Phase 3: items fall off bottom (heights stay at minimums).
            let total_variable = ih.saturating_sub(fixed);
            let det_full = det_max * 2;
            let ov_full  = OV_MAX * 2;

            let (both_det, both_ov) = if total_variable >= det_full + ov_full {
                (det_full, ov_full)
            } else if total_variable >= DET_MIN * 2 + ov_full {
                (total_variable - ov_full, ov_full)
            } else if total_variable >= DET_MIN * 2 + OV_MIN * 2 {
                (DET_MIN * 2, total_variable - DET_MIN * 2)
            } else {
                let d = total_variable.min(DET_MIN * 2);
                (d, total_variable.saturating_sub(d))
            };

            // Clamp to minimums: the pool calculation drives compression through
            // the normal phase range; below minimum, take_h handles falloff.
            let effective_det_h = (both_det / 2).max(DET_MIN).min(det_max);
            let effective_ov_h  = (both_ov  / 2).clamp(OV_MIN, OV_MAX);

            // Allocate rows top-to-bottom using take_exact for all waveform rows:
            // each waveform shows at its computed height or disappears entirely.
            // This prevents partial heights below the minimum (e.g. a 3-row
            // detail area where the tick rows leave only 1 waveform row).
            let mut rem = ih;
            // take: allocate up to n rows (partial ok — used for 1-row fixed items).
            // take_consume: show at full height or not at all, but always consume
            //   up to n rows so freed space cannot cause lower items to reappear.
            let take         = |rem: &mut u16, n: u16| -> u16 { let h = (*rem).min(n); *rem -= h; h };
            let take_consume = |rem: &mut u16, n: u16| -> u16 {
                let actual = if *rem >= n { n } else { 0 };
                *rem = rem.saturating_sub(n);
                actual
            };
            let hh = [
                take(&mut rem, 1),                      // 0: global bar
                take(&mut rem, 1),                      // 1: detail info bar
                take_consume(&mut rem, effective_det_h), // 2: detail A
                take_consume(&mut rem, effective_det_h), // 3: detail B
                take(&mut rem, 1),                      // 4: notif A
                take(&mut rem, 1),                      // 5: info A
                take_consume(&mut rem, effective_ov_h),  // 6: overview A
                take(&mut rem, 1),                      // 7: notif B
                take(&mut rem, 1),                      // 8: info B
                take_consume(&mut rem, effective_ov_h),  // 9: overview B
                rem,                                    // 10: spacer (leftover)
            ];

            let c = Layout::default()
                .direction(Direction::Vertical)
                .constraints(hh.map(Constraint::Length))
                .split(inner);
            let (area_detail_info, area_detail_a, area_detail_b,
                 area_notif_a, area_info_a, area_overview_a,
                 area_notif_b, area_info_b, area_overview_b,
                 area_global) = (c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[0]);

            // Update renderer dimensions from layout.
            {
                let w = area_detail_a.width as usize;
                let h = area_detail_a.height as usize;
                if w > 0 && h > 0 {
                    shared_renderer.cols.store(w, Ordering::Relaxed);
                    shared_renderer.rows.store(h.saturating_sub(1), Ordering::Relaxed);
                }
            }

            // Detail info bar
            {
                let nudge_label = match d0.as_ref().or(d1.as_ref()).map(|d| d.nudge_mode) {
                    Some(NudgeMode::Warp) => "  [WARP]",
                    _ => "  [JUMP]",
                };
                let spc_label = if space_held { "  [SPC]" } else { "" };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        format!("  zoom:{}s  lat:{}ms{}{}", zoom_secs, audio_latency_ms, nudge_label, spc_label),
                        Style::default().fg(Color::Rgb(60, 60, 60)),
                    ))),
                    area_detail_info,
                );
            }

            let label_style = Style::default().fg(Color::Rgb(40, 60, 100));
            let notif_bg    = Style::default().bg(Color::Rgb(20, 20, 38));

            // Pre-compute shared tick row: separate beat grids for deck A and deck B.
            let (shared_tick_a, shared_tick_b): (Vec<u8>, Vec<u8>) = {
                let w = area_detail_a.width as usize;
                let centre_col = ((w as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
                    .clamp(0, w.saturating_sub(1));
                let tick_for = |buf: &Arc<BrailleBuffer>, deck: Option<&Deck>, pos: usize| -> Vec<u8> {
                    let Some(deck) = deck else { return vec![0u8; w]; };
                    let analysing = deck.tempo.analysis_hash.is_none() || !deck.tempo.bpm_established;
                    let mvs = if buf.samples_per_col > 0 {
                        pos as f64 - centre_col as f64 * buf.samples_per_col as f64
                    } else { 0.0 };
                    compute_tick_display(w, buf.samples_per_col, mvs, analysing,
                        deck.tempo.base_bpm, deck.audio.sample_rate, deck.tempo.offset_ms)
                };
                let pos_a = render[0].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
                let pos_b = render[1].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
                (tick_for(&buf_a, d0.as_ref(), pos_a), tick_for(&buf_b, d1.as_ref(), pos_b))
            };

            // ---- Deck A ----
            if let (Some(deck), Some(rs)) = (&mut d0, &render[0]) {
                let content = notification_line_for_deck(deck);
                let mut spans = vec![Span::styled("A ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_a.width);
                frame.render_widget(Paragraph::new(info), area_info_a);
                let (ov, bar_cols, bar_times) = overview_for_deck(deck, area_overview_a, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                deck.display.overview_rect  = area_overview_a;
                deck.display.last_bar_cols  = bar_cols;
                deck.display.last_bar_times = bar_times;
                frame.render_widget(Paragraph::new(ov), area_overview_a);
                render_detail_waveform(frame, &buf_a, deck, area_detail_a, &display_cfg, rs.display_pos_samp, Some((&shared_tick_a, &shared_tick_b)));
            } else {
                let mut spans = vec![Span::styled("A ", label_style)];
                if let Some(ref s) = loading_label[0] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_a.width)), area_info_a);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_a)), area_overview_a);
                render_detail_empty(frame, area_detail_a, &display_cfg, Some((&shared_tick_a, &shared_tick_b)));
            }

            // ---- Deck B ----
            if let (Some(deck), Some(rs)) = (&mut d1, &render[1]) {
                let content = notification_line_for_deck(deck);
                let mut spans = vec![Span::styled("B ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_b.width);
                frame.render_widget(Paragraph::new(info), area_info_b);
                let (ov, bar_cols, bar_times) = overview_for_deck(deck, area_overview_b, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                deck.display.overview_rect  = area_overview_b;
                deck.display.last_bar_cols  = bar_cols;
                deck.display.last_bar_times = bar_times;
                frame.render_widget(Paragraph::new(ov), area_overview_b);
                render_detail_waveform(frame, &buf_b, deck, area_detail_b, &display_cfg, rs.display_pos_samp, None);
            } else {
                let mut spans = vec![Span::styled("B ", label_style)];
                if let Some(ref s) = loading_label[1] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_b.width)), area_info_b);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_b)), area_overview_b);
                render_detail_empty(frame, area_detail_b, &display_cfg, None);
            }

            // ---- Global status bar ----
            {
                let global_line = if pending_quit {
                    Line::from(vec![
                        Span::styled("  Track is playing — quit? ", Style::default().fg(Color::Yellow)),
                        Span::styled("[y] yes  ", Style::default().fg(Color::White)),
                        Span::styled("[Esc/n] cancel", Style::default().fg(Color::DarkGray)),
                    ])
                } else if let Some(ref n) = global_notification {
                    let color = match n.style {
                        NotificationStyle::Info    => Color::DarkGray,
                        NotificationStyle::Warning => Color::Yellow,
                        NotificationStyle::Error   => Color::Red,
                    };
                    Line::from(Span::styled(n.message.clone(), Style::default().fg(color)))
                } else {
                    Line::from(Span::styled(
                        format!("  {}", browser_dir.display()),
                        Style::default().fg(Color::Rgb(50, 50, 50)),
                    ))
                };
                frame.render_widget(Paragraph::new(global_line), area_global);
            }

            // Help popup
            if help_open {
                const HELP: &str = "\
Space+Z / Space+C    play / pause  Deck 1 / Deck 2
Z / C                load file into Deck 1 / Deck 2
1/2  q/w             Deck 1 jump ±4/±8 bars    Space+1/2  q/w  ±1/±4 beats
3/4  e/r             Deck 2 jump ±4/±8 bars    Space+3/4  e/r  ±1/±4 beats
a / z                Deck 1 nudge fwd / bwd     d / c  Deck 2
|                    toggle nudge mode: jump (10ms) / warp (±10%)
j/m  k/,             Deck 1/2 level up/down     Space+J/M  K/,  snap 100%/0%
7/u  i/8             Deck 1/2 filter sweep      Space+7/u  i/8  filter reset
s/x  f/v             Deck 1/2 BPM ±0.1          S/X  F/V  base BPM ±0.01
@  /  ~              Deck 1/2 tempo reset
b  /  n              Deck 1/2 tap BPM            B / N  metronome
'  /  #              Deck 1/2 BPM redetect
!  Q                 Deck 1 offset ±10ms         £  E  Deck 2
[  /  ]              latency ±10ms
-  /  =              zoom in / out
{  /  }              detail height decrease / increase
O                    toggle waveform fill / outline
P                    cycle spectral colour palette
`                    refresh terminal
?                    toggle this help
Esc                  close this / quit";
                let popup_w = 75u16;
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

        // Put both decks back after render.
        decks[0] = d0;
        decks[1] = d1;

        // Single event handler — all actions work regardless of which deck is loaded.
        while event::poll(Duration::ZERO)? {
            match event::read()? {
            Event::Mouse(mouse_event) => {
                if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                    let col = mouse_event.column as usize;
                    let row = mouse_event.row as usize;
                    for slot in 0..2 {
                        if let Some(ref d) = decks[slot] {
                            let rect = d.display.overview_rect;
                            if col >= rect.x as usize && col < (rect.x + rect.width) as usize
                                && row >= rect.y as usize && row < (rect.y + rect.height) as usize
                            {
                                let click_col = col - rect.x as usize;
                                let target_secs = d.display.last_bar_cols.iter()
                                    .zip(d.display.last_bar_times.iter())
                                    .filter(|(c, _)| **c <= click_col)
                                    .last()
                                    .map(|(_, t)| *t)
                                    .unwrap_or(0.0);
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(target_secs);
                                } else {
                                    d.audio.seek_handle.seek_to(target_secs);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Event::Key(key) => {
                // Ctrl-C: unconditional quit.
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    for slot in 0..2 {
                        if let Some(ref d) = decks[slot] {
                            d.audio.player.stop();
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { offset_ms: d.tempo.offset_ms, ..entry });
                                }
                            }
                        }
                    }
                    cache.save();
                    return Ok(());
                }
                // Space modifier: track held state for chords.
                if key.code == KeyCode::Char(' ') {
                    space_saw_event_this_frame = true;
                    match key.kind {
                        KeyEventKind::Press | KeyEventKind::Repeat => {
                            if !space_repeat_suppressed { space_held = true; }
                        }
                        KeyEventKind::Release => {
                            space_held = false;
                            space_repeat_suppressed = false;
                        }
                    }
                }
                // Nudge and mode toggle — handled for all key kinds (Release must be detected).
                match key.kind {
                    KeyEventKind::Press
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeModeToggle) =>
                    {
                        let new_mode = decks.iter().flatten().next()
                            .map(|d| match d.nudge_mode {
                                NudgeMode::Jump => NudgeMode::Warp,
                                NudgeMode::Warp => NudgeMode::Jump,
                            })
                            .unwrap_or(NudgeMode::Jump);
                        for slot in 0..2 {
                            if let Some(ref mut d) = decks[slot] {
                                if d.nudge != 0 {
                                    d.nudge = 0;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                }
                                d.nudge_mode = new_mode;
                            }
                        }
                    }
                    // Cue handlers come before nudge so that Space+nudge-key resolves to
                    // cue (via SpaceChord lookup) rather than nudge.
                    // Press guard requires space_held to avoid firing on bare nudge-key presses.
                    KeyEventKind::Press
                        if space_held && keymap.get(&KeyBinding::SpaceChord(key.code)) == Some(&Action::Deck1CuePlay) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            if let Some(cue_samp) = d.cue_sample {
                                let was_playing = !d.audio.player.is_paused();
                                d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                if was_playing {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    d.display.smooth_display_samp = (cue_samp + latency_samps) as f64;
                                    if d.filter_offset != 0 { d.audio.filter_state_reset.store(true, Ordering::Relaxed); }
                                    d.audio.player.play();
                                } else {
                                    d.display.smooth_display_samp = cue_samp as f64;
                                }
                            }
                            space_held = false;
                            space_repeat_suppressed = true;
                        }
                    }
                    KeyEventKind::Press
                        if space_held && keymap.get(&KeyBinding::SpaceChord(key.code)) == Some(&Action::Deck2CuePlay) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            if let Some(cue_samp) = d.cue_sample {
                                let was_playing = !d.audio.player.is_paused();
                                d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                if was_playing {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    d.display.smooth_display_samp = (cue_samp + latency_samps) as f64;
                                    if d.filter_offset != 0 { d.audio.filter_state_reset.store(true, Ordering::Relaxed); }
                                    d.audio.player.play();
                                } else {
                                    d.display.smooth_display_samp = cue_samp as f64;
                                }
                            }
                            space_held = false;
                            space_repeat_suppressed = true;
                        }
                    }
                    // Deck 1 nudge
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck1NudgeBackward) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current - 0.010).max(0.0);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_a);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = -1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 0.9);
                                }
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck1NudgeForward) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current + 0.010).min(d.total_duration.as_secs_f64());
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_a);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = 1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 1.1);
                                }
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::Deck1NudgeBackward) | Some(&Action::Deck1NudgeForward)) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            if d.nudge_mode == NudgeMode::Warp {
                                d.nudge = 0;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            }
                        }
                    }
                    // Deck 2 nudge
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck2NudgeBackward) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current - 0.010).max(0.0);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_b);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = -1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 0.9);
                                }
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck2NudgeForward) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current + 0.010).min(d.total_duration.as_secs_f64());
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_b);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = 1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 1.1);
                                }
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::Deck2NudgeBackward) | Some(&Action::Deck2NudgeForward)) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            if d.nudge_mode == NudgeMode::Warp {
                                d.nudge = 0;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            }
                        }
                    }
                    _ => {}
                }
                // All other actions fire on Press only.
                if key.kind == KeyEventKind::Press {
                    // Dismiss any open overlay before processing any other action.
                    if help_open {
                        help_open = false;
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Quit) {
                            suppress_quit_until = Some(Instant::now() + Duration::from_millis(300));
                        }
                        continue 'tui;
                    }
                    // Quit confirmation intercept — y/Enter confirms, anything else cancels.
                    if pending_quit {
                        pending_quit = false;
                        if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                            for slot in 0..2 {
                                if let Some(ref d) = decks[slot] {
                                    d.audio.player.stop();
                                    if let Some(ref hash) = d.tempo.analysis_hash {
                                        if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                            cache.set(hash.clone(), CacheEntry { offset_ms: d.tempo.offset_ms, ..entry });
                                        }
                                    }
                                }
                            }
                            cache.set_latency(audio_latency_ms);
                            cache.save();
                            return Ok(());
                        }
                        continue 'tui;
                    }
                    // BPM confirmation intercept — check both decks.
                    let mut bpm_intercepted = false;
                    for slot in 0..2 {
                        if let Some(ref mut d) = decks[slot] {
                            if let Some((hash, p_bpm, p_offset, _)) = d.tempo.pending_bpm.take() {
                                if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                                    d.tempo.bpm = p_bpm;
                                    d.tempo.base_bpm = p_bpm;
                                    d.tempo.offset_ms = (p_offset as f64 / 10.0).round() as i64 * 10;
                                    d.tempo.bpm_established = true;
                                    d.audio.player.set_speed(1.0);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                                    cache.set(hash.clone(), CacheEntry { bpm: d.tempo.bpm, offset_ms: d.tempo.offset_ms, name: d.filename.clone(), cue_sample: d.cue_sample });
                                    cache.save();
                                    d.tempo.analysis_hash = Some(hash);
                                }
                                // Any key dismisses the confirmation.
                                bpm_intercepted = true;
                                break;
                            }
                        }
                    }
                    if bpm_intercepted { continue 'tui; }

                    let action = if space_held && key.code != KeyCode::Char(' ') {
                        if let Some(a) = keymap.get(&KeyBinding::SpaceChord(key.code)) {
                            space_held = false;
                            space_repeat_suppressed = true;
                            Some(a)
                        } else {
                            keymap.get(&KeyBinding::Key(key.code))
                        }
                    } else {
                        keymap.get(&KeyBinding::Key(key.code))
                    };
                    match action {
                    Some(Action::Quit) => {
                        if suppress_quit_until.take().map_or(false, |until| Instant::now() < until) {
                            continue 'tui;
                        }
                        let any_playing = decks.iter().flatten().any(|d| !d.audio.player.is_paused());
                        if any_playing && !pending_quit {
                            pending_quit = true;
                            continue 'tui;
                        }
                        for slot in 0..2 {
                            if let Some(ref d) = decks[slot] {
                                d.audio.player.stop();
                                if let Some(ref hash) = d.tempo.analysis_hash {
                                    if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                        cache.set(hash.clone(), CacheEntry { offset_ms: d.tempo.offset_ms, ..entry });
                                    }
                                }
                            }
                        }
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                        return Ok(());
                    }
                    Some(Action::Deck1OpenBrowser) | Some(Action::Deck2OpenBrowser) => {
                        let target = if action == Some(&Action::Deck1OpenBrowser) { 0 } else { 1 };
                        // Save cache state but leave the player running — the deck continues
                        // playing while the browser is open. stop() is deferred to the
                        // Selected branch; returning without selecting leaves the deck intact.
                        if let Some(ref d) = decks[target] {
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { offset_ms: d.tempo.offset_ms, ..entry });
                                }
                            }
                        }
                        cache.save();
                        match run_browser(terminal, browser_dir.clone())? {
                            (BrowserResult::ReturnToPlayer, cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                cache.save();
                            }
                            (BrowserResult::Selected(path), cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                cache.save();
                                // Stop and drop the outgoing deck.
                                if let Some(ref d) = decks[target] {
                                    d.audio.player.stop();
                                }
                                decks[target] = None;
                                // Cancel any in-progress load for this slot and start the new one.
                                pending_loads[target] = Some(start_load(&path));
                            }
                            (BrowserResult::Quit, cwd) => {
                                *browser_dir = cwd;
                                cache.set_last_browser_path(browser_dir);
                                for slot in 0..2 { if let Some(ref d) = decks[slot] { d.audio.player.stop(); } }
                                cache.save();
                                return Ok(());
                            }
                        }
                        continue 'tui;
                    }
                    Some(Action::Deck1PlayPause) => {
                        if let Some(ref d) = decks[0] {
                            if d.audio.player.is_paused() {
                                if d.filter_offset != 0 {
                                    d.audio.filter_state_reset.store(true, Ordering::Relaxed);
                                }
                                d.audio.player.play();
                            } else {
                                d.audio.player.pause();
                            }
                        }
                    }
                    Some(Action::Deck2PlayPause) => {
                        if let Some(ref d) = decks[1] {
                            if d.audio.player.is_paused() {
                                if d.filter_offset != 0 {
                                    d.audio.filter_state_reset.store(true, Ordering::Relaxed);
                                }
                                d.audio.player.play();
                            } else {
                                d.audio.player.pause();
                            }
                        }
                    }
                    Some(Action::Deck1LevelUp)   => { if let Some(ref mut d) = decks[0] { d.volume = (d.volume + 0.05).min(1.0); d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck1LevelDown)  => { if let Some(ref mut d) = decks[0] { d.volume = (d.volume - 0.05).max(0.0); d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck1LevelMax)   => { if let Some(ref mut d) = decks[0] { d.volume = 1.0; d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck1LevelMin)   => { if let Some(ref mut d) = decks[0] { d.volume = 0.0; d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck2LevelUp)    => { if let Some(ref mut d) = decks[1] { d.volume = (d.volume + 0.05).min(1.0); d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck2LevelDown)  => { if let Some(ref mut d) = decks[1] { d.volume = (d.volume - 0.05).max(0.0); d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck2LevelMax)   => { if let Some(ref mut d) = decks[1] { d.volume = 1.0; d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck2LevelMin)   => { if let Some(ref mut d) = decks[1] { d.volume = 0.0; d.audio.player.set_volume(d.volume); } }
                    Some(Action::Deck1MetronomeToggle) => {
                        if let Some(ref mut d) = decks[0] {
                            d.metronome_mode = !d.metronome_mode;
                            d.last_metro_beat = if d.metronome_mode {
                                let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
                                let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                                    - d.tempo.offset_ms as i128 * 1_000_000;
                                Some(ns.div_euclid(beat_period.as_nanos() as i128))
                            } else { None };
                        }
                    }
                    Some(Action::Deck2MetronomeToggle) => {
                        if let Some(ref mut d) = decks[1] {
                            d.metronome_mode = !d.metronome_mode;
                            d.last_metro_beat = if d.metronome_mode {
                                let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
                                let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                                    - d.tempo.offset_ms as i128 * 1_000_000;
                                Some(ns.div_euclid(beat_period.as_nanos() as i128))
                            } else { None };
                        }
                    }
                    Some(Action::Deck1RedetectBpm) => {
                        if let Some(ref mut d) = decks[0] {
                            if d.tempo.pending_bpm.is_some() {
                                d.tempo.pending_bpm = None;
                            } else if d.tempo.redetecting {
                                let (_, dead_rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                d.tempo.background_rx = Some(std::mem::replace(&mut d.tempo.bpm_rx, dead_rx));
                                d.tempo.redetecting = false;
                                d.tempo.analysis_hash = d.tempo.redetect_saved_hash.take();
                            } else if d.tempo.analysis_hash.is_some() {
                                if let Some(bg_rx) = d.tempo.background_rx.take() {
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.bpm_rx = bg_rx;
                                    d.tempo.redetecting = true;
                                } else {
                                    let mono_bg = Arc::clone(&d.audio.mono);
                                    let (tx, rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                    let hash_bg = d.tempo.analysis_hash.clone().unwrap_or_default();
                                    let sr_bg = d.audio.sample_rate;
                                    thread::spawn(move || {
                                        if let Ok(bpm) = detect_bpm(&mono_bg, sr_bg) {
                                            let _ = tx.send((hash_bg, bpm, 0, true));
                                        }
                                    });
                                    d.tempo.bpm_rx = rx;
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.redetecting = true;
                                }
                            }
                        }
                    }
                    Some(Action::Deck2RedetectBpm) => {
                        if let Some(ref mut d) = decks[1] {
                            if d.tempo.pending_bpm.is_some() {
                                d.tempo.pending_bpm = None;
                            } else if d.tempo.redetecting {
                                let (_, dead_rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                d.tempo.background_rx = Some(std::mem::replace(&mut d.tempo.bpm_rx, dead_rx));
                                d.tempo.redetecting = false;
                                d.tempo.analysis_hash = d.tempo.redetect_saved_hash.take();
                            } else if d.tempo.analysis_hash.is_some() {
                                if let Some(bg_rx) = d.tempo.background_rx.take() {
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.bpm_rx = bg_rx;
                                    d.tempo.redetecting = true;
                                } else {
                                    let mono_bg = Arc::clone(&d.audio.mono);
                                    let (tx, rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                    let hash_bg = d.tempo.analysis_hash.clone().unwrap_or_default();
                                    let sr_bg = d.audio.sample_rate;
                                    thread::spawn(move || {
                                        if let Ok(bpm) = detect_bpm(&mono_bg, sr_bg) {
                                            let _ = tx.send((hash_bg, bpm, 0, true));
                                        }
                                    });
                                    d.tempo.bpm_rx = rx;
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.redetecting = true;
                                }
                            }
                        }
                    }
                    Some(Action::Help)            => { help_open = true; }
                    Some(Action::TerminalRefresh)  => { let _ = terminal.clear(); }
                    Some(Action::LatencyDecrease)  => {
                        audio_latency_ms = (audio_latency_ms - 10).max(0);
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                    }
                    Some(Action::LatencyIncrease)  => {
                        audio_latency_ms = (audio_latency_ms + 10).min(250);
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                    }
                    Some(Action::Deck1FilterIncrease) => { if let Some(ref mut d) = decks[0] { d.filter_offset = (d.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterDecrease) => { if let Some(ref mut d) = decks[0] { d.filter_offset = (d.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterReset)    => { if let Some(ref mut d) = decks[0] { d.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterIncrease) => { if let Some(ref mut d) = decks[1] { d.filter_offset = (d.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterDecrease) => { if let Some(ref mut d) = decks[1] { d.filter_offset = (d.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterReset)    => { if let Some(ref mut d) = decks[1] { d.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::WaveformStyle) => {
                        let s = shared_renderer.style.load(Ordering::Relaxed);
                        shared_renderer.style.store(1 - s, Ordering::Relaxed);
                    }
                    Some(Action::PaletteCycle) => {
                        let next = decks.iter().flatten().next()
                            .map(|d| (d.display.palette_idx + 1) % SPECTRAL_PALETTES.len())
                            .unwrap_or(0);
                        for slot in 0..2 {
                            if let Some(ref mut d) = decks[slot] { d.display.palette_idx = next; }
                        }
                    }
                    Some(Action::Deck1OffsetIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            apply_offset_step(d, 10);
                        }
                    }
                    Some(Action::Deck1OffsetDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            apply_offset_step(d, -10);
                        }
                    }
                    Some(Action::Deck2OffsetIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            apply_offset_step(d, 10);
                        }
                    }
                    Some(Action::Deck2OffsetDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            apply_offset_step(d, -10);
                        }
                    }
                    Some(Action::ZoomOut) => { if zoom_idx > 0 { zoom_idx -= 1; } }
                    Some(Action::ZoomIn)  => { if zoom_idx + 1 < ZOOM_LEVELS.len() { zoom_idx += 1; } }
                    Some(Action::HeightDecrease) => { if detail_height > DET_MIN as usize { detail_height -= 1; } }
                    Some(Action::HeightIncrease) => { if detail_height < max_det_h { detail_height += 1; } }
                    Some(Action::Deck1BpmIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.bpm = (d.tempo.bpm + 0.1).min(240.0);
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                        }
                    }
                    Some(Action::Deck1BpmDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.bpm = (d.tempo.bpm - 0.1).max(40.0);
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                        }
                    }
                    Some(Action::Deck2BpmIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.bpm = (d.tempo.bpm + 0.1).min(240.0);
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                        }
                    }
                    Some(Action::Deck2BpmDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.bpm = (d.tempo.bpm - 0.1).max(40.0);
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                        }
                    }
                    Some(Action::Deck1BaseBpmIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.base_bpm = (d.tempo.base_bpm + 0.01).min(240.0);
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck1BaseBpmDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.base_bpm = (d.tempo.base_bpm - 0.01).max(40.0);
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck2BaseBpmIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.base_bpm = (d.tempo.base_bpm + 0.01).min(240.0);
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck2BaseBpmDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.base_bpm = (d.tempo.base_bpm - 0.01).max(40.0);
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.tempo.bpm_established = true;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            anchor_beat_grid_to_cue(d);
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                    cache.set(hash.clone(), CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck1JumpForward4b)   => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  16); } }
                    Some(Action::Deck1JumpBackward4b)  => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -16); } }
                    Some(Action::Deck1JumpForward8b)   => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  32); } }
                    Some(Action::Deck1JumpBackward8b)  => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -32); } }
                    Some(Action::Deck1JumpForward1bt)  => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  1); } }
                    Some(Action::Deck1JumpBackward1bt) => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -1); } }
                    Some(Action::Deck1JumpForward4bt)  => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  4); } }
                    Some(Action::Deck1JumpBackward4bt) => { if let Some(ref d) = decks[0] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -4); } }
                    Some(Action::Deck2JumpForward4b)   => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  16); } }
                    Some(Action::Deck2JumpBackward4b)  => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -16); } }
                    Some(Action::Deck2JumpForward8b)   => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  32); } }
                    Some(Action::Deck2JumpBackward8b)  => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -32); } }
                    Some(Action::Deck2JumpForward1bt)  => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  1); } }
                    Some(Action::Deck2JumpBackward1bt) => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -1); } }
                    Some(Action::Deck2JumpForward4bt)  => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  4); } }
                    Some(Action::Deck2JumpBackward4bt) => { if let Some(ref d) = decks[1] { do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -4); } }
                    Some(Action::Deck1TempoReset) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                        }
                    }
                    Some(Action::Deck2TempoReset) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.audio.player.set_speed(1.0);
                            shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                        }
                    }
                    Some(Action::Deck1BpmTap) => {
                        if let Some(ref mut d) = decks[0] {
                            let now = Instant::now();
                            if let Some(last) = d.tap.last_tap_wall {
                                if now.duration_since(last).as_secs_f64() > 2.0 { d.tap.tap_times.clear(); }
                            }
                            let display_samp = render[0].as_ref().map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                            d.tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                            d.tap.last_tap_wall = Some(now);
                            if d.tap.tap_times.len() >= 8 {
                                let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
                                let tapped_offset = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
                                let speed_ratio = d.tempo.bpm / d.tempo.base_bpm;
                                d.tempo.base_bpm = tapped_bpm;
                                d.tempo.bpm = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
                                d.tempo.offset_ms = tapped_offset;
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            }
                        }
                    }
                    Some(Action::Deck2BpmTap) => {
                        if let Some(ref mut d) = decks[1] {
                            let now = Instant::now();
                            if let Some(last) = d.tap.last_tap_wall {
                                if now.duration_since(last).as_secs_f64() > 2.0 { d.tap.tap_times.clear(); }
                            }
                            let display_samp = render[1].as_ref().map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                            d.tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                            d.tap.last_tap_wall = Some(now);
                            if d.tap.tap_times.len() >= 8 {
                                let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
                                let tapped_offset = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
                                let speed_ratio = d.tempo.bpm / d.tempo.base_bpm;
                                d.tempo.base_bpm = tapped_bpm;
                                d.tempo.bpm = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
                                d.tempo.offset_ms = tapped_offset;
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            }
                        }
                    }
                    Some(Action::Deck1Cue) => {
                        if let Some(ref mut d) = decks[0] {
                            if d.audio.player.is_paused() {
                                let raw_samp = d.display.smooth_display_samp as usize;
                                d.cue_sample = Some(raw_samp);
                                anchor_beat_grid_to_cue(d);
                                if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                    let entry = cache.get(hash.as_str()).cloned()
                                        .unwrap_or(CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, name: d.filename.clone(), cue_sample: None });
                                    cache.set(hash.clone(), CacheEntry { cue_sample: d.cue_sample, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck2Cue) => {
                        if let Some(ref mut d) = decks[1] {
                            if d.audio.player.is_paused() {
                                let raw_samp = d.display.smooth_display_samp as usize;
                                d.cue_sample = Some(raw_samp);
                                anchor_beat_grid_to_cue(d);
                                if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                    let entry = cache.get(hash.as_str()).cloned()
                                        .unwrap_or(CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, name: d.filename.clone(), cue_sample: None });
                                    cache.set(hash.clone(), CacheEntry { cue_sample: d.cue_sample, offset_ms: d.tempo.offset_ms, ..entry });
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck1CuePlay) => {
                        if let Some(ref mut d) = decks[0] {
                            if let Some(cue_samp) = d.cue_sample {
                                let was_playing = !d.audio.player.is_paused();
                                d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                if was_playing {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    d.display.smooth_display_samp = (cue_samp + latency_samps) as f64;
                                    if d.filter_offset != 0 { d.audio.filter_state_reset.store(true, Ordering::Relaxed); }
                                    d.audio.player.play();
                                } else {
                                    d.display.smooth_display_samp = cue_samp as f64;
                                }
                            }
                        }
                    }
                    Some(Action::Deck2CuePlay) => {
                        if let Some(ref mut d) = decks[1] {
                            if let Some(cue_samp) = d.cue_sample {
                                let was_playing = !d.audio.player.is_paused();
                                d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                if was_playing {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    d.display.smooth_display_samp = (cue_samp + latency_samps) as f64;
                                    if d.filter_offset != 0 { d.audio.filter_state_reset.store(true, Ordering::Relaxed); }
                                    d.audio.player.play();
                                } else {
                                    d.display.smooth_display_samp = cue_samp as f64;
                                }
                            }
                        }
                    }
                    Some(Action::Deck1NudgeBackward) | Some(Action::Deck1NudgeForward)
                    | Some(Action::Deck2NudgeBackward) | Some(Action::Deck2NudgeForward)
                    | Some(Action::NudgeModeToggle) => {}
                    None => {}
                    }
                } // end if Press
            }
            _ => {}
            }
        }

        thread::sleep(frame_dur.saturating_sub(frame_start.elapsed()));
    }
}

fn service_deck_frame(
    slot: usize,
    decks: &mut [Option<Deck>; 2],
    col_secs: f64,
    frame_dur: Duration,
    elapsed: f64,
    mixer: &rodio::mixer::Mixer,
    shared_renderer: &SharedDetailRenderer,
    cache: &mut Cache,
    audio_latency_ms: i64,
) {
    let Some(ref mut d) = decks[slot] else { return; };

    // Auto-reject pending BPM confirmation after 15 seconds.
    if let Some((_, _, _, received_at)) = &d.tempo.pending_bpm {
        if received_at.elapsed().as_secs() >= 15 {
            d.tempo.pending_bpm = None;
        }
    }



    // Expire per-deck active notification.
    if d.active_notification.as_ref().map_or(false, |n| Instant::now() >= n.expires) {
        d.active_notification = None;
    }

    // Poll BPM detection results.
    if let Ok((hash, new_bpm, new_offset, is_fresh)) = d.tempo.bpm_rx.try_recv() {
        if !is_fresh || !d.tempo.bpm_established {
            d.tempo.bpm      = new_bpm;
            d.tempo.base_bpm = new_bpm;
            shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
            d.tempo.offset_ms = (new_offset as f64 / 10.0).round() as i64 * 10;
            // Restore cue_sample from cache if present.
            d.cue_sample = cache.get(hash.as_str()).and_then(|e| e.cue_sample);
            cache.set(hash.clone(), CacheEntry { bpm: d.tempo.bpm, offset_ms: d.tempo.offset_ms, name: d.filename.clone(), cue_sample: d.cue_sample });
            cache.save();
            d.tempo.analysis_hash      = Some(hash);
            if !is_fresh || d.tempo.redetecting { d.tempo.bpm_established = true; }
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
        } else {
            d.tempo.analysis_hash      = Some(hash.clone());
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
            d.tempo.pending_bpm        = Some((hash, new_bpm, new_offset, Instant::now()));
        }
    }

    // Real audio position.
    let pos_raw  = d.audio.seek_handle.position.load(Ordering::Relaxed);
    let pos_samp = pos_raw / d.audio.seek_handle.channels as usize;
    let total_mono_samps = d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize;

    // End-of-track: pause and reset to start.
    let at_end = pos_samp >= total_mono_samps;
    if at_end && !d.audio.player.is_paused() {
        d.audio.player.pause();
        d.audio.seek_handle.seek_direct(0.0);
        d.display.smooth_display_samp = 0.0;
        return;
    }

    // Advance smooth display position.
    if !d.audio.player.is_paused() {
        // Include warp-nudge speed factor so the display tracks the audio speed exactly.
        let speed = (d.tempo.bpm / d.tempo.base_bpm) as f64 * (1.0 + d.nudge as f64 * 0.1);
        // Use nominal frame duration rather than measured elapsed to avoid systematic drift:
        // thread::sleep overshoots, so elapsed is consistently larger than frame_dur.
        d.display.smooth_display_samp += frame_dur.as_secs_f64() * d.audio.sample_rate as f64 * speed;
    } else if d.nudge != 0 {
        // Paused with warp nudge: drift display and sync actual audio position for scrubbing.
        d.display.smooth_display_samp = (d.display.smooth_display_samp
            + elapsed * d.audio.sample_rate as f64 * d.nudge as f64 * 0.1)
            .clamp(0.0, total_mono_samps as f64);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
        // Fire a scrub snippet once per half-column advance.
        let scrub_spc = if slot == 0 {
            shared_renderer.shared_a.lock().unwrap().samples_per_col
        } else {
            shared_renderer.shared_b.lock().unwrap().samples_per_col
        };
        let half_samples_per_col = (scrub_spc / 2).max(1);
        if scrub_spc > 0
            && (d.display.smooth_display_samp - d.display.last_scrub_samp).abs() >= half_samples_per_col as f64
        {
            scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                        d.audio.sample_rate, d.display.smooth_display_samp as usize, half_samples_per_col);
            d.display.last_scrub_samp = d.display.smooth_display_samp;
        }
    }

    // Drift correction.
    let drift = d.display.smooth_display_samp - pos_samp as f64;
    let large_drift = drift.abs() > d.audio.sample_rate as f64 * 0.5;
    let paused_snap  = d.audio.player.is_paused() && d.nudge == 0 && drift.abs() > 1.0;
    if large_drift || paused_snap {
        // Snap to nearest half-column so sub_col is stable after seeks.
        let speed = (d.tempo.bpm / d.tempo.base_bpm) as f64;
        let col_samp_f64 = col_secs * d.audio.sample_rate as f64 * speed;
        let half_col = col_samp_f64 / 2.0;
        d.display.smooth_display_samp = if half_col > 0.0 {
            (pos_samp as f64 / half_col).round() * half_col
        } else {
            pos_samp as f64
        };
    } else if !d.audio.player.is_paused() {
        d.display.smooth_display_samp -= drift * 0.05;
    }

    // Metronome: fire from buffer write position so the click arrives at the speaker on the beat.
    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
    let metro_beat_index = {
        let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
            - d.tempo.offset_ms as i128 * 1_000_000;
        ns.div_euclid(beat_period.as_nanos() as i128)
    };
    if d.metronome_mode && !d.audio.player.is_paused() {
        if d.last_metro_beat != Some(metro_beat_index) {
            play_click_tone(mixer, d.audio.sample_rate);
            d.last_metro_beat = Some(metro_beat_index);
        }
    } else {
        d.last_metro_beat = None;
    }

    // Tap session timeout: finalise BPM when the user stops tapping.
    let tap_active_now = !d.tap.tap_times.is_empty()
        && d.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
    if d.tap.was_tap_active && !tap_active_now && d.tap.tap_times.len() >= 8 {
        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
        let tapped_offset  = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
        let speed_ratio    = d.tempo.bpm / d.tempo.base_bpm;
        d.tempo.base_bpm   = tapped_bpm;
        d.tempo.bpm        = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
        d.tempo.offset_ms  = tapped_offset;
        d.tempo.bpm_established = true;
        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
        shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
        if let Some(ref hash) = d.tempo.analysis_hash {
            cache.set(hash.clone(), CacheEntry { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms, name: d.filename.clone(), cue_sample: d.cue_sample });
            cache.save();
        }
    }
    d.tap.was_tap_active = tap_active_now;

    // Spectrum analyser: chars every half beat, background glow every 8 beats.
    let analysing   = d.tempo.analysis_hash.is_none();
    let half_period = if analysing { Duration::from_millis(500) } else { beat_period / 2 };
    let bar_period  = beat_period * 8;
    let chars_due   = d.spectrum.last_update.map_or(true,    |t| t.elapsed() >= half_period);
    let bg_due      = d.spectrum.last_bg_update.map_or(true, |t| t.elapsed() >= bar_period);
    if chars_due || bg_due {
        let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
        let display_pos_samp = (d.display.smooth_display_samp - latency_correction).max(0.0) as usize;
        let (new_chars, new_bg) = compute_spectrum(&d.audio.mono, display_pos_samp, d.audio.sample_rate, d.filter_offset);
        if chars_due {
            d.spectrum.chars = new_chars;
            for i in 0..16 { d.spectrum.bg_accum[i] |= new_bg[i]; }
            d.spectrum.bg = d.spectrum.bg_accum;
            d.spectrum.last_update = Some(Instant::now());
        }
        if bg_due {
            d.spectrum.bg_accum = [false; 16];
            d.spectrum.last_bg_update = Some(Instant::now());
        }
    }
}

// ---------------------------------------------------------------------------
// Per-deck helpers
// ---------------------------------------------------------------------------

/// After a BPM change, re-anchor `offset_ms` so the beat grid stays aligned to
/// the cue position. With no cue set this is a no-op.
fn anchor_beat_grid_to_cue(d: &mut Deck) {
    if let Some(cue_samp) = d.cue_sample {
        let cue_ms = cue_samp as f64 / d.audio.sample_rate as f64 * 1000.0;
        let beat_period_ms = 60_000.0 / d.tempo.base_bpm as f64;
        let raw = cue_ms.rem_euclid(beat_period_ms);
        d.tempo.offset_ms = (raw / 10.0).round() as i64 * 10;
    }
}

/// Apply a ±10ms offset step and keep the display position in sync when paused.
///
/// The display delta uses the raw `delta_ms` step — never `new_offset - old_offset`.
/// Those two values differ when `rem_euclid` wraps the offset across a beat boundary,
/// which would shift `smooth_display_samp` by nearly a full period and trigger a
/// spurious waveform rerender.
fn apply_offset_step(d: &mut Deck, delta_ms: i64) {
    d.tempo.offset_ms += delta_ms;
    let period = (60_000.0 / d.tempo.base_bpm as f64 / 10.0).round() as i64 * 10;
    d.tempo.offset_ms = d.tempo.offset_ms.rem_euclid(period);
    if d.audio.player.is_paused() {
        let delta_samp = delta_ms as f64 / 1000.0 * d.audio.sample_rate as f64;
        d.display.smooth_display_samp = (d.display.smooth_display_samp + delta_samp).max(0.0);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
    }
}

// ---------------------------------------------------------------------------
// Per-deck render helpers (free functions used by terminal.draw closures)
// ---------------------------------------------------------------------------

fn notification_line_for_deck(deck: &Deck) -> Line<'static> {
    if let Some((_, p_bpm, _, received_at)) = &deck.tempo.pending_bpm {
        let secs_left = 15u64.saturating_sub(received_at.elapsed().as_secs());
        let yellow = Style::default().fg(Color::Yellow);
        let countdown_style = if secs_left <= 5 {
            Style::default().fg(Color::Red)
        } else {
            yellow
        };
        Line::from(vec![
            Span::styled(format!("BPM detected: {p_bpm:.2}  [y] accept  [n] reject  ("), yellow),
            Span::styled(format!("{secs_left}s"), countdown_style),
            Span::styled(")", yellow),
        ])
    } else if let Some(ref n) = deck.active_notification {
        let color = match n.style {
            NotificationStyle::Info    => Color::DarkGray,
            NotificationStyle::Warning => Color::Yellow,
            NotificationStyle::Error   => Color::Red,
        };
        Line::from(Span::styled(n.message.clone(), Style::default().fg(color)))
    } else {
        let (_, _, (tr, tg, tb)) = SPECTRAL_PALETTES[deck.display.palette_idx];
        let muted = |c: u8| (c as f32 * 0.55) as u8;
        Line::from(Span::styled(
            deck.track_name.clone(),
            Style::default().fg(Color::Rgb(muted(tr), muted(tg), muted(tb))),
        ))
    }
}

fn info_line_for_deck(
    deck: &Deck,
    frame_count: usize,
    beat_on: bool,
    analysing: bool,
    _label_style: Style,
    bar_width: u16,
) -> Line<'static> {
    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let play_icon = if deck.audio.player.is_paused() { "⏸" } else { "▶" };
    let nudge_str = match deck.nudge {
        1  => "  ▶nudge",
        -1 => "  ◀nudge",
        _  => "",
    };
    let tap_active = !deck.tap.tap_times.is_empty()
        && deck.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
    let tap_str = if tap_active {
        format!("  tap:{}", deck.tap.tap_times.len())
    } else {
        String::new()
    };
    let dim = Style::default().fg(Color::DarkGray);
    if analysing {
        return Line::from(vec![
            Span::styled(format!("{play_icon}  "), dim),
            Span::styled(
                format!("[analysing {}]", SPINNER[(frame_count / 6) % SPINNER.len()]),
                dim,
            ),
        ]);
    }
    let beat_style = if beat_on {
        Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0))
    } else {
        dim
    };
    let adjusted = (deck.tempo.bpm - deck.tempo.base_bpm).abs() >= 0.05;

    // --- Left group ---
    let left_spans: Vec<Span<'static>> = {
        let mut spans = vec![Span::styled(format!("{play_icon}  "), dim)];
        let unconfirmed = !deck.tempo.bpm_established;
        let red_style = Style::default().fg(Color::Red);
        if adjusted {
            spans.push(Span::styled(format!("{:.2} ", deck.tempo.base_bpm), dim));
            spans.push(Span::styled("(", dim));
            if unconfirmed {
                spans.push(Span::styled(format!("~{:.2}", deck.tempo.bpm), red_style));
            } else {
                spans.push(Span::styled(format!("{:.2}", deck.tempo.bpm), beat_style));
            }
            spans.push(Span::styled(")", dim));
        } else if unconfirmed {
            spans.push(Span::styled(format!("~{:.2}", deck.tempo.base_bpm), red_style));
        } else {
            spans.push(Span::styled(format!("{:.2}", deck.tempo.base_bpm), beat_style));
        }
        if deck.metronome_mode {
            spans.push(Span::styled("\u{266A}", Style::default().fg(Color::Red)));
        }
        spans.push(Span::styled(format!("  {:+}ms", deck.tempo.offset_ms), dim));
        if !tap_str.is_empty() {
            spans.push(Span::styled(tap_str.clone(), dim));
        }
        spans
    };

    // --- Right group ---
    let mut right_spans: Vec<Span<'static>> = Vec::new();
    if !nudge_str.is_empty() {
        right_spans.push(Span::styled(nudge_str.to_string(), dim));
    }
    const LEVEL_BARS: [char; 8] = ['▁','▂','▃','▄','▅','▆','▇','█'];
    let level_char = LEVEL_BARS[((deck.volume * 7.0).round() as usize).min(7)];
    let bracket_style = Style::default().fg(Color::Rgb(140, 140, 140));
    right_spans.push(Span::styled("  level:", dim));
    right_spans.push(Span::styled("\u{2595}", bracket_style));
    right_spans.push(Span::styled(level_char.to_string(), Style::default().fg(Color::Rgb(120, 100, 0))));
    right_spans.push(Span::styled("\u{258F}", bracket_style));
    {
        let stopband: Option<(bool, usize)> = if deck.filter_offset != 0 {
            let n = deck.filter_offset.unsigned_abs() as usize;
            let is_lpf = deck.filter_offset < 0;
            let cutoff_char = if is_lpf { 16 - n } else { n };
            Some((is_lpf, cutoff_char))
        } else {
            None
        };
        right_spans.push(Span::styled("  \u{2595}".to_string(), dim));
        for i in 0..16 {
            let ch = deck.spectrum.chars[i].to_string();
            let in_stopband = stopband.map_or(false, |(is_lpf, cutoff_char)| {
                if is_lpf { i >= cutoff_char } else { i < cutoff_char }
            });
            let style = if in_stopband {
                if ch != "\u{2800}" {
                    Style::default().fg(Color::Rgb(120, 100, 0)).bg(Color::Rgb(50, 50, 50))
                } else {
                    Style::default().bg(Color::Rgb(50, 50, 50))
                }
            } else if ch != "\u{2800}" {
                Style::default().fg(Color::Yellow).bg(Color::Rgb(40, 33, 0))
            } else if deck.spectrum.bg[i] {
                Style::default().bg(Color::Rgb(40, 33, 0))
            } else {
                Style::default()
            };
            right_spans.push(Span::styled(ch, style));
        }
        right_spans.push(Span::styled("\u{258F}".to_string(), dim));
    }

    // Spacer: fill gap between left and right groups.
    let bar_w = bar_width as usize;
    let left_w: usize = left_spans.iter().map(|s| s.content.chars().count()).sum();
    let right_w: usize = right_spans.iter().map(|s| s.content.chars().count()).sum();
    let spacer_w = bar_w.saturating_sub(left_w + right_w).max(1);
    let mut info_spans = left_spans;
    info_spans.push(Span::raw(" ".repeat(spacer_w)));
    info_spans.extend(right_spans);
    Line::from(info_spans)
}

fn overview_for_deck(
    deck: &Deck,
    rect: ratatui::layout::Rect,
    display_samp: f64,
    analysing: bool,
    warning_active: bool,
    warn_beat_on: bool,
) -> (Vec<Line<'static>>, Vec<usize>, Vec<f64>) {
    let overview_width  = rect.width  as usize;
    let overview_height = rect.height as usize;
    let total_peaks = deck.audio.waveform.peaks.len();
    let playhead_frac = if deck.total_duration.is_zero() {
        0.0
    } else {
        (display_samp / deck.audio.sample_rate as f64 / deck.total_duration.as_secs_f64()).clamp(0.0, 1.0)
    };
    let playhead_col = ((playhead_frac * overview_width as f64).round() as usize)
        .min(overview_width.saturating_sub(1));
    let cue_col: Option<usize> = deck.cue_sample.map(|samp| {
        let frac = (samp as f64 / deck.audio.sample_rate as f64
            / deck.total_duration.as_secs_f64()).clamp(0.0, 1.0);
        ((frac * overview_width as f64).round() as usize)
            .min(overview_width.saturating_sub(1))
    });

    let (ov_peaks_hires, ov_bass_hires): (Vec<(f32, f32)>, Vec<f32>) = (0..overview_width * 2)
        .map(|col| {
            let idx = (col * total_peaks / (overview_width * 2).max(1)).min(total_peaks.saturating_sub(1));
            (deck.audio.waveform.peaks[idx], deck.audio.waveform.bass_ratio[idx])
        })
        .unzip();
    let hires_buf = render_braille(&ov_peaks_hires, overview_height, overview_width * 2, false);
    let ov_braille: Vec<Vec<u8>> = hires_buf.iter()
        .map(|row| (0..overview_width).map(|c| (row[c * 2] & 0x47) | (row[c * 2 + 1] & 0xB8)).collect())
        .collect();
    let ov_bass: Vec<f32> = (0..overview_width)
        .map(|c| (ov_bass_hires[c * 2] + ov_bass_hires[c * 2 + 1]) / 2.0)
        .collect();
    let (bar_cols, bar_times, bars_per_tick): (Vec<usize>, Vec<f64>, u32) = if !analysing {
        bar_tick_cols(deck.tempo.base_bpm as f64, deck.tempo.offset_ms, deck.total_duration.as_secs_f64(), overview_width)
    } else {
        (Vec::new(), Vec::new(), 4)
    };
    let legend: String = if !analysing {
        format!("{} bars", bars_per_tick)
    } else {
        String::new()
    };
    let legend_start = overview_width.saturating_sub(legend.len());

    let ov_lines: Vec<Line<'static>> = ov_braille
        .into_iter()
        .enumerate()
        .map(|(r, row)| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_color = Color::Reset;
            for (c, byte) in row.into_iter().enumerate() {
                let (color, ch) = if r == 0 && c >= legend_start && !legend.is_empty() {
                    let lch = legend.chars().nth(c - legend_start).unwrap_or(' ');
                    (Color::DarkGray, lch)
                } else if c == playhead_col && cue_col == Some(c) {
                    if r == 0 || r + 1 == overview_height {
                        (Color::Green, '\u{28FF}')
                    } else {
                        (Color::White, '\u{28FF}')
                    }
                } else if c == playhead_col {
                    (Color::White, '\u{28FF}')
                } else if cue_col == Some(c) {
                    (Color::Green, '\u{28FF}')
                } else if bar_cols.contains(&c) {
                    if warn_beat_on {
                        (Color::Rgb(120, 60, 60), '│')
                    } else if warning_active {
                        (Color::Rgb(40, 20, 20), '│')
                    } else {
                        (Color::DarkGray, '│')
                    }
                } else {
                    let r_val = ov_bass[c];
                    let (_, (br, bg, bb), (tr, tg, tb)) = SPECTRAL_PALETTES[deck.display.palette_idx];
                    let spectral = Color::Rgb(
                        (br as f32 * r_val + tr as f32 * (1.0 - r_val)) as u8,
                        (bg as f32 * r_val + tg as f32 * (1.0 - r_val)) as u8,
                        (bb as f32 * r_val + tb as f32 * (1.0 - r_val)) as u8,
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

    (ov_lines, bar_cols, bar_times)
}

fn notification_line_empty() -> Line<'static> {
    Line::from(Span::styled(
        "no track — press z to open the file browser",
        Style::default().fg(Color::Rgb(60, 60, 60)),
    ))
}

fn info_line_empty(bar_width: u16) -> Line<'static> {
    let dim = Style::default().fg(Color::Rgb(60, 60, 60));
    let left  = Span::styled("⏸  ---  +0ms", dim);
    let right = Span::styled("zoom:---", dim);
    let lw = left.content.chars().count();
    let rw = right.content.chars().count();
    let spacer = " ".repeat((bar_width as usize).saturating_sub(lw + rw).max(1));
    Line::from(vec![left, Span::raw(spacer), right])
}

fn overview_empty(rect: ratatui::layout::Rect) -> Vec<Line<'static>> {
    let w = rect.width as usize;
    let h = rect.height as usize;
    if w == 0 || h == 0 { return vec![]; }

    // Zero-amplitude peaks (flat line) at double width for half-col resolution.
    let hires: Vec<(f32, f32)> = vec![(0.0f32, 0.0f32); w * 2];
    let hires_buf = render_braille(&hires, h, w * 2, false);
    let ov_braille: Vec<Vec<u8>> = hires_buf.iter()
        .map(|row| (0..w).map(|c| (row[c * 2] & 0x47) | (row[c * 2 + 1] & 0xB8)).collect())
        .collect();

    // 120 BPM ticks over a 5-minute dummy duration.
    let (bar_cols, _, _) = bar_tick_cols(120.0, 0, 300.0, w);

    let wave_color = Color::Rgb(35, 35, 55);
    let tick_color = Color::DarkGray;

    ov_braille.into_iter().map(|row| {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_color = Color::Reset;
        for (c, byte) in row.into_iter().enumerate() {
            let (color, ch) = if bar_cols.contains(&c) {
                (tick_color, '│')
            } else {
                (wave_color, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
            };
            if color != run_color {
                if !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
                }
                run_color = color;
            }
            run.push(ch);
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, Style::default().fg(run_color)));
        }
        Line::from(spans)
    }).collect()
}

fn render_detail_empty(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    display_cfg: &DisplayConfig,
    shared_tick: Option<(&[u8], &[u8])>,
) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 { return; }

    let centre_col = ((w as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
        .clamp(0, w.saturating_sub(1));

    let waveform_rows = if shared_tick.is_some() { h.saturating_sub(1) } else { h };
    let wave_color   = Color::Rgb(35, 35, 55);
    let centre_color = Color::Rgb(60, 60, 60);

    let empty_row = vec![0u8; w];
    let mut lines: Vec<Line<'static>> = (0..waveform_rows).map(|_r| {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_color = Color::Reset;
        for (c, &byte) in empty_row.iter().enumerate() {
            let (color, ch) = if c == centre_col {
                (centre_color, '⣿')
            } else {
                (wave_color, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
            };
            if color != run_color {
                if !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
                }
                run_color = color;
            }
            run.push(ch);
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, Style::default().fg(run_color)));
        }
        Line::from(spans)
    }).collect();

    // Shared tick row (only for deck A slot).
    if let Some((tick_a, tick_b)) = shared_tick {
        let tick_color = Color::Rgb(60, 60, 60);
        let display_row = compose_shared_tick_row(tick_a, tick_b, w);
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_color = Color::Reset;
        for c in 0..w {
            let byte = display_row[c];
            let (color, ch) = if c == centre_col {
                (centre_color, '⣿')
            } else if byte != 0 {
                (tick_color, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
            } else {
                (tick_color, ' ')
            };
            if color != run_color {
                if !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
                }
                run_color = color;
            }
            run.push(ch);
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, Style::default().fg(run_color)));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

// Compose the braille display row for the shared tick strip.
// Each tick occupies two adjacent characters: a main char at column c and a spillover into c+1.
// Deck A (up): 1 tip dot (row 1, sub-col position) + 3-wide base (row 2).
// Deck B (down): 3-wide base (row 3) + 1 tip dot (row 4, sub-col position).
// Left/right sub-column is determined by bit 0 of the raw tick byte (0x47=left, 0xB8=right).
fn compose_shared_tick_row(tick_a: &[u8], tick_b: &[u8], width: usize) -> Vec<u8> {
    let mut row = vec![0u8; width];
    for c in 0..width {
        let a = tick_a.get(c).copied().unwrap_or(0);
        let b = tick_b.get(c).copied().unwrap_or(0);
        if a != 0 {
            let (c_pat, c1_pat): (u8, u8) = if a & 0x01 != 0 {
                (0x1A, 0x02) // left sub-col: tip=row1-col1, base=row2 cols 0,1 + col0 of c+1
            } else {
                (0x10, 0x13) // right sub-col: base=row2 col1 + cols 0,1 of c+1, tip=row1-col0 of c+1
            };
            row[c] |= c_pat;
            if c + 1 < width { row[c + 1] |= c1_pat; }
        }
        if b != 0 {
            let (c_pat, c1_pat): (u8, u8) = if b & 0x01 != 0 {
                (0xA4, 0x04) // left sub-col: base=row3 cols 0,1 + col0 of c+1, tip=row4-col1
            } else {
                (0x20, 0x64) // right sub-col: base=row3 col1 + cols 0,1 of c+1, tip=row4-col0 of c+1
            };
            row[c] |= c_pat;
            if c + 1 < width { row[c + 1] |= c1_pat; }
        }
    }
    row
}

fn compute_tick_display(
    detail_width:    usize,
    samples_per_col: usize,
    marker_view_start: f64,
    analysing:       bool,
    base_bpm:        f32,
    sample_rate:     u32,
    offset_ms:       i64,
) -> Vec<u8> {
    if analysing || samples_per_col == 0 {
        return vec![0u8; detail_width];
    }
    let mut row = vec![0u8; detail_width];
    let samples_per_col    = samples_per_col as f64;
    let half_samples_per_col = samples_per_col / 2.0;
    let beat_period_samp   = 60.0 / base_bpm as f64 * sample_rate as f64;
    let offset_samp        = offset_ms as f64 / 1000.0 * sample_rate as f64;
    let view_end           = marker_view_start + detail_width as f64 * samples_per_col;
    let n_start            = ((marker_view_start - offset_samp) / beat_period_samp).floor() as i64 - 1;
    let mut t_samp         = offset_samp + n_start as f64 * beat_period_samp;
    while t_samp <= view_end {
        let disp_half = ((t_samp - marker_view_start) / half_samples_per_col).round() as i64;
        if disp_half >= 0 {
            let col = (disp_half / 2) as usize;
            if col < detail_width {
                row[col] = if disp_half % 2 != 0 { 0xB8 } else { 0x47 };
            }
        }
        t_samp += beat_period_samp;
    }
    row
}

fn render_detail_waveform(
    frame: &mut ratatui::Frame,
    buf: &Arc<BrailleBuffer>,
    deck: &mut Deck,
    detail_area: ratatui::layout::Rect,
    display_cfg: &DisplayConfig,
    display_pos_samp: usize,
    shared_tick: Option<(&[u8], &[u8])>,
) {
    let detail_width      = detail_area.width  as usize;
    let detail_panel_rows = detail_area.height as usize;
    let buf = Arc::clone(buf);
    let centre_col = ((detail_width as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
        .clamp(0, detail_width.saturating_sub(1));

    let half_col_samp_global: f64 = buf.samples_per_col as f64 / 2.0;
    let mut sub_col = false;
    let viewport_start: Option<usize> = if buf.buf_cols >= detail_width && buf.samples_per_col > 0 {
        let delta = display_pos_samp as i64 - buf.anchor_sample as i64;
        let delta_half = (delta as f64 / half_col_samp_global).round() as i64;
        sub_col = delta_half % 2 != 0;
        let delta_cols = delta_half.div_euclid(2);
        let viewport_offset = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
        let need = if sub_col { detail_width + 1 } else { detail_width };
        if viewport_offset >= 0 && (viewport_offset as usize) + need <= buf.buf_cols {
            let start = viewport_offset as usize;
            deck.display.last_viewport_start = start;
            Some(start)
        } else {
            None
        }
    } else {
        None
    };

    // Marker positions (cue) are computed from the exact display position.
    // viewport_start uses the half-column-quantized anchor (correct for the discrete
    // buffer grid); using that quantized value here would leave a residual that flips
    // disp_half by 1 whenever the display position moves — visible as marker wobble.
    let marker_view_start: f64 = if buf.samples_per_col > 0 {
        display_pos_samp as f64 - centre_col as f64 * buf.samples_per_col as f64
    } else {
        0.0
    };

    let cue_screen_col: Option<usize> = if buf.samples_per_col > 0 {
        deck.cue_sample.and_then(|samp| {
            let disp_half = ((samp as f64 - marker_view_start) / half_col_samp_global).round() as i64;
            if disp_half >= 0 {
                let col = (disp_half / 2) as usize;
                if col < detail_width { Some(col) } else { None }
            } else {
                None
            }
        })
    } else {
        None
    };

    // Waveform rows occupy the full panel minus the shared tick row (if present).
    let waveform_rows = if shared_tick.is_some() {
        detail_panel_rows.saturating_sub(1)
    } else {
        detail_panel_rows
    };

    let mut detail_lines: Vec<Line<'static>> = (0..waveform_rows)
        .map(|r| {
            // buf_r maps directly: row 0 → buffer row 0 (no top tick row to skip).
            let buf_r = r;
            let shifted: Option<Vec<u8>>;
            let row_slice: Option<&[u8]>;
            shifted = if sub_col {
                viewport_start.and_then(|start| {
                    buf.grid.get(buf_r).map(|row| {
                        (0..detail_width).map(|c| shift_braille_half(row[start + c], row[start + c + 1])).collect()
                    })
                })
            } else { None };
            row_slice = if sub_col {
                shifted.as_deref()
            } else {
                viewport_start.and_then(|start| buf.grid.get(buf_r).map(|row| &row[start..start + detail_width]))
            };
            let _ = &shifted;
            let is_edge_row = r == 0 || r + 1 == waveform_rows;
            let row = match row_slice {
                None => return Line::from(Span::raw("\u{2800}".repeat(detail_width))),
                Some(s) => s,
            };
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_color = Color::Reset;
            for (c, &byte) in row.iter().enumerate() {
                let (color, ch) = if c == centre_col && cue_screen_col == Some(c) {
                    if is_edge_row {
                        (Color::Green, '\u{28FF}')
                    } else {
                        (Color::White, '\u{28FF}')
                    }
                } else if c == centre_col {
                    (Color::White, '\u{28FF}')
                } else if cue_screen_col == Some(c) {
                    (Color::Green, '\u{28FF}')
                } else {
                    (Color::Cyan, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                };
                if color != run_color {
                    if !run.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
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

    // Shared tick row (bottom of deck A panel only).
    if let Some((tick_a, tick_b)) = shared_tick {
        let display_row = compose_shared_tick_row(tick_a, tick_b, detail_width);
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_color = Color::Reset;
        for c in 0..detail_width {
            let byte = display_row[c];
            let (color, ch) = if c == centre_col && cue_screen_col == Some(c) {
                (Color::Green, '\u{28FF}')
            } else if c == centre_col {
                (Color::White, '\u{28FF}')
            } else if cue_screen_col == Some(c) {
                (Color::Green, '\u{28FF}')
            } else if byte != 0 {
                (Color::Gray, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
            } else {
                (Color::Gray, ' ')
            };
            if color != run_color {
                if !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
                }
                run_color = color;
            }
            run.push(ch);
        }
        if !run.is_empty() {
            spans.push(Span::styled(run, Style::default().fg(run_color)));
        }
        detail_lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(detail_lines), detail_area);
}

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

// ---------------------------------------------------------------------------
// Deck structs
// ---------------------------------------------------------------------------

struct StopOnDrop(Arc<AtomicBool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
}

struct DeckAudio {
    player: Player,
    seek_handle: SeekHandle,
    mono: Arc<Vec<f32>>,
    waveform: Arc<WaveformData>,
    sample_rate: u32,
    filter_offset_shared: Arc<AtomicI32>,
    filter_state_reset: Arc<AtomicBool>,
}

struct TempoState {
    bpm: f32,
    base_bpm: f32,
    offset_ms: i64,
    bpm_rx: mpsc::Receiver<(String, f32, i64, bool)>,
    analysis_hash: Option<String>,
    bpm_established: bool,
    pending_bpm: Option<(String, f32, i64, Instant)>,
    redetecting: bool,
    redetect_saved_hash: Option<String>,
    background_rx: Option<mpsc::Receiver<(String, f32, i64, bool)>>,
}

struct TapState {
    tap_times: Vec<f64>,
    last_tap_wall: Option<Instant>,
    was_tap_active: bool,
}

struct DisplayState {
    smooth_display_samp: f64,
    last_scrub_samp: f64,
    last_viewport_start: usize,
    overview_rect: ratatui::layout::Rect,
    last_bar_cols: Vec<usize>,
    last_bar_times: Vec<f64>,
    palette_idx: usize,
}

struct SpectrumState {
    chars: [char; 16],
    bg: [bool; 16],
    bg_accum: [bool; 16],
    last_update: Option<Instant>,
    last_bg_update: Option<Instant>,
}

// ---------------------------------------------------------------------------
// Shared two-deck detail waveform renderer
// ---------------------------------------------------------------------------

/// A single background thread that produces two `BrailleBuffer`s — one per
/// deck — each at a `col_samp` scaled by that deck's `bpm / base_bpm` ratio.
/// Scaling by the playback speed means ticks placed at `base_bpm` sample
/// spacing appear at `bpm`-spaced columns, so the tick grids of two decks at
/// the same effective BPM are visually identical.
struct SharedDetailRenderer {
    cols:           Arc<AtomicUsize>,
    rows:           Arc<AtomicUsize>,
    zoom_at:        Arc<AtomicUsize>,
    style:          Arc<AtomicUsize>,
    sample_rate_a:  Arc<AtomicUsize>,
    sample_rate_b:  Arc<AtomicUsize>,
    /// `(bpm / base_bpm) × 65536`, updated on every BPM-changing action.
    speed_ratio_a:  Arc<AtomicUsize>,
    speed_ratio_b:  Arc<AtomicUsize>,
    waveform_a:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    waveform_b:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    display_pos_a:  Arc<AtomicUsize>,
    display_pos_b:  Arc<AtomicUsize>,
    channels_a:     Arc<AtomicUsize>,
    channels_b:     Arc<AtomicUsize>,
    /// Incremented each time a new track is loaded into the slot; signals the
    /// background thread to recompute immediately rather than waiting for drift.
    load_gen_a:     Arc<AtomicUsize>,
    load_gen_b:     Arc<AtomicUsize>,
    shared_a:       Arc<Mutex<Arc<BrailleBuffer>>>,
    shared_b:       Arc<Mutex<Arc<BrailleBuffer>>>,
    _stop_guard:    StopOnDrop,
}

impl SharedDetailRenderer {
    fn new(zoom_idx: usize) -> Self {
        let cols           = Arc::new(AtomicUsize::new(0));
        let rows           = Arc::new(AtomicUsize::new(0));
        let zoom_at        = Arc::new(AtomicUsize::new(zoom_idx));
        let style          = Arc::new(AtomicUsize::new(0));
        let sample_rate_a  = Arc::new(AtomicUsize::new(44100));
        let sample_rate_b  = Arc::new(AtomicUsize::new(44100));
        let speed_ratio_a  = Arc::new(AtomicUsize::new(65536)); // 1.0 × 65536
        let speed_ratio_b  = Arc::new(AtomicUsize::new(65536));
        let waveform_a     = Arc::new(Mutex::new(None::<Arc<WaveformData>>));
        let waveform_b     = Arc::new(Mutex::new(None::<Arc<WaveformData>>));
        let display_pos_a  = Arc::new(AtomicUsize::new(0));
        let display_pos_b  = Arc::new(AtomicUsize::new(0));
        let channels_a     = Arc::new(AtomicUsize::new(1));
        let channels_b     = Arc::new(AtomicUsize::new(1));
        let load_gen_a     = Arc::new(AtomicUsize::new(0));
        let load_gen_b     = Arc::new(AtomicUsize::new(0));
        let shared_a: Arc<Mutex<Arc<BrailleBuffer>>> =
            Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
        let shared_b: Arc<Mutex<Arc<BrailleBuffer>>> =
            Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
        let stop       = Arc::new(AtomicBool::new(false));
        let stop_guard = StopOnDrop(Arc::clone(&stop));

        {
            let cols_bg      = Arc::clone(&cols);
            let rows_bg      = Arc::clone(&rows);
            let zoom_bg      = Arc::clone(&zoom_at);
            let style_bg     = Arc::clone(&style);
            let sr_a_bg      = Arc::clone(&sample_rate_a);
            let sr_b_bg      = Arc::clone(&sample_rate_b);
            let ratio_a_bg   = Arc::clone(&speed_ratio_a);
            let ratio_b_bg   = Arc::clone(&speed_ratio_b);
            let wf_a_bg      = Arc::clone(&waveform_a);
            let wf_b_bg      = Arc::clone(&waveform_b);
            let pos_a_bg     = Arc::clone(&display_pos_a);
            let pos_b_bg     = Arc::clone(&display_pos_b);
            let ch_a_bg      = Arc::clone(&channels_a);
            let ch_b_bg      = Arc::clone(&channels_b);
            let gen_a_bg     = Arc::clone(&load_gen_a);
            let gen_b_bg     = Arc::clone(&load_gen_b);
            let shared_a_bg  = Arc::clone(&shared_a);
            let shared_b_bg  = Arc::clone(&shared_b);
            let stop_bg      = Arc::clone(&stop);

            thread::spawn(move || {
                let mut last_cols      = 0usize;
                let mut last_rows      = 0usize;
                let mut last_zoom      = usize::MAX;
                let mut last_style     = usize::MAX;
                let mut last_col_samp_a = 0usize;
                let mut last_col_samp_b = 0usize;
                let mut last_anchor_a  = 0usize;
                let mut last_anchor_b  = 0usize;
                let mut last_gen_a     = usize::MAX;
                let mut last_gen_b     = usize::MAX;

                loop {
                    if stop_bg.load(Ordering::Relaxed) { break; }

                    let cols = cols_bg.load(Ordering::Relaxed);
                    let rows = rows_bg.load(Ordering::Relaxed);
                    if cols == 0 || rows == 0 {
                        thread::sleep(Duration::from_millis(8));
                        continue;
                    }

                    let zoom      = zoom_bg.load(Ordering::Relaxed).min(ZOOM_LEVELS.len() - 1);
                    let zoom_secs = ZOOM_LEVELS[zoom] as f64;
                    let sr_a      = sr_a_bg.load(Ordering::Relaxed);
                    let sr_b      = sr_b_bg.load(Ordering::Relaxed);
                    let ratio_a   = ratio_a_bg.load(Ordering::Relaxed) as f64 / 65536.0;
                    let ratio_b   = ratio_b_bg.load(Ordering::Relaxed) as f64 / 65536.0;
                    // col_samp scaled by speed ratio so column grid is in playback-time space.
                    let col_samp_a = ((zoom_secs * sr_a as f64 * ratio_a) as usize / cols).max(1);
                    let col_samp_b = ((zoom_secs * sr_b as f64 * ratio_b) as usize / cols).max(1);

                    let ch_a   = ch_a_bg.load(Ordering::Relaxed).max(1);
                    let ch_b   = ch_b_bg.load(Ordering::Relaxed).max(1);
                    let pos_a  = pos_a_bg.load(Ordering::Relaxed) / ch_a;
                    let pos_b  = pos_b_bg.load(Ordering::Relaxed) / ch_b;

                    let drift_a = if last_col_samp_a > 0 {
                        pos_a.abs_diff(last_anchor_a) / last_col_samp_a
                    } else { usize::MAX };
                    let drift_b = if last_col_samp_b > 0 {
                        pos_b.abs_diff(last_anchor_b) / last_col_samp_b
                    } else { usize::MAX };

                    let style = style_bg.load(Ordering::Relaxed);
                    let gen_a = gen_a_bg.load(Ordering::Relaxed);
                    let gen_b = gen_b_bg.load(Ordering::Relaxed);
                    let must_recompute = cols != last_cols
                        || rows != last_rows
                        || zoom != last_zoom
                        || style != last_style
                        || col_samp_a != last_col_samp_a
                        || col_samp_b != last_col_samp_b
                        || drift_a >= cols * 3 / 4
                        || drift_b >= cols * 3 / 4
                        || gen_a != last_gen_a
                        || gen_b != last_gen_b;

                    if must_recompute {
                        let buf_cols = cols * 5;

                        let wf_a: Option<Arc<WaveformData>> = wf_a_bg.lock().unwrap().clone();
                        let wf_b: Option<Arc<WaveformData>> = wf_b_bg.lock().unwrap().clone();

                        let anchor_a = (pos_a / col_samp_a) * col_samp_a;
                        let anchor_b = (pos_b / col_samp_b) * col_samp_b;

                        let buf_a = Arc::new(BrailleBuffer {
                            grid: render_braille(
                                &peaks_for_slot(&wf_a, anchor_a, col_samp_a, buf_cols),
                                rows, buf_cols, style == 1,
                            ),
                            buf_cols,
                            anchor_sample:   anchor_a,
                            samples_per_col: col_samp_a,
                        });
                        let buf_b = Arc::new(BrailleBuffer {
                            grid: render_braille(
                                &peaks_for_slot(&wf_b, anchor_b, col_samp_b, buf_cols),
                                rows, buf_cols, style == 1,
                            ),
                            buf_cols,
                            anchor_sample:   anchor_b,
                            samples_per_col: col_samp_b,
                        });

                        *shared_a_bg.lock().unwrap() = buf_a;
                        *shared_b_bg.lock().unwrap() = buf_b;

                        last_cols       = cols;
                        last_rows       = rows;
                        last_zoom       = zoom;
                        last_style      = style;
                        last_col_samp_a = col_samp_a;
                        last_col_samp_b = col_samp_b;
                        last_anchor_a   = anchor_a;
                        last_anchor_b   = anchor_b;
                        last_gen_a      = gen_a;
                        last_gen_b      = gen_b;
                    }

                    thread::sleep(Duration::from_millis(8));
                }
            });
        }

        SharedDetailRenderer {
            cols, rows, zoom_at, style,
            sample_rate_a, sample_rate_b,
            speed_ratio_a, speed_ratio_b,
            waveform_a, waveform_b,
            display_pos_a, display_pos_b,
            channels_a, channels_b,
            load_gen_a, load_gen_b,
            shared_a, shared_b,
            _stop_guard: stop_guard,
        }
    }

    fn set_deck(&self, slot: usize, wf: Arc<WaveformData>, channels: u16, sample_rate: u32) {
        match slot {
            0 => {
                *self.waveform_a.lock().unwrap() = Some(wf);
                self.channels_a.store(channels as usize, Ordering::Relaxed);
                self.sample_rate_a.store(sample_rate as usize, Ordering::Relaxed);
                self.speed_ratio_a.store(65536, Ordering::Relaxed); // reset to 1.0 on load
                self.load_gen_a.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                *self.waveform_b.lock().unwrap() = Some(wf);
                self.channels_b.store(channels as usize, Ordering::Relaxed);
                self.sample_rate_b.store(sample_rate as usize, Ordering::Relaxed);
                self.speed_ratio_b.store(65536, Ordering::Relaxed);
                self.load_gen_b.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn store_speed_ratio(&self, slot: usize, bpm: f32, base_bpm: f32) {
        let ratio = ((bpm / base_bpm) as f64 * 65536.0) as usize;
        match slot {
            0 => self.speed_ratio_a.store(ratio, Ordering::Relaxed),
            _ => self.speed_ratio_b.store(ratio, Ordering::Relaxed),
        }
    }

}

fn peaks_for_slot(
    wf: &Option<Arc<WaveformData>>,
    anchor: usize,
    col_samp: usize,
    buf_cols: usize,
) -> Vec<(f32, f32)> {
    let Some(wf) = wf else {
        return vec![(0.0, 0.0); buf_cols];
    };
    let mono = &wf.mono;
    (0..buf_cols).map(|c| {
        let offset    = c as i64 - (buf_cols / 2) as i64;
        let raw_start = anchor as i64 + offset * col_samp as i64;
        if raw_start < 0 {
            return (1.0, -1.0);
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
    }).collect()
}

struct Deck {
    filename: String,
    track_name: String,
    total_duration: Duration,
    volume: f32,
    filter_offset: i32,
    nudge: i8,
    nudge_mode: NudgeMode,
    metronome_mode: bool,
    last_metro_beat: Option<i128>,
    active_notification: Option<Notification>,
    cue_sample: Option<usize>,

    audio: DeckAudio,
    tempo: TempoState,
    tap: TapState,
    display: DisplayState,
    spectrum: SpectrumState,
}

impl Deck {
    fn new(
        filename: String,
        track_name: String,
        total_duration: Duration,
        audio: DeckAudio,
        bpm_rx: mpsc::Receiver<(String, f32, i64, bool)>,
    ) -> Self {
        Deck {
            filename,
            track_name,
            total_duration,
            volume: 1.0,
            filter_offset: 0,
            nudge: 0,
            nudge_mode: NudgeMode::Jump,
            metronome_mode: false,
            last_metro_beat: None,
            active_notification: None,
            cue_sample: None,
            audio,
            tempo: TempoState {
                bpm: 120.0,
                base_bpm: 120.0,
                offset_ms: 0,
                bpm_rx,
                analysis_hash: None,
                bpm_established: false,
                pending_bpm: None,
                redetecting: false,
                redetect_saved_hash: None,
                background_rx: None,
            },
            tap: TapState {
                tap_times: Vec::new(),
                last_tap_wall: None,
                was_tap_active: false,
            },
            display: DisplayState {
                smooth_display_samp: 0.0,
                last_scrub_samp: -1.0,
                last_viewport_start: 0,
                overview_rect: ratatui::layout::Rect::default(),
                last_bar_cols: Vec::new(),
                last_bar_times: Vec::new(),
                palette_idx: 0,
            },
            spectrum: SpectrumState {
                chars: ['\u{2800}'; 16],
                bg: [false; 16],
                bg_accum: [false; 16],
                last_update: None,
                last_bg_update: None,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum NudgeMode { Jump, Warp }

#[allow(dead_code)]
enum NotificationStyle { Info, Warning, Error }

struct Notification {
    message: String,
    style:   NotificationStyle,
    expires: Instant,
}

// ---------------------------------------------------------------------------
// Keyboard mapping
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Quit, Help, TerminalRefresh,
    ZoomIn, ZoomOut, HeightIncrease, HeightDecrease,
    LatencyIncrease, LatencyDecrease,
    WaveformStyle, PaletteCycle,
    NudgeModeToggle,
    // Deck 1
    Deck1PlayPause, Deck1OpenBrowser,
    Deck1LevelUp, Deck1LevelDown, Deck1LevelMax, Deck1LevelMin,
    Deck1FilterIncrease, Deck1FilterDecrease, Deck1FilterReset,
    Deck1BpmTap, Deck1MetronomeToggle, Deck1RedetectBpm,
    Deck1BpmIncrease, Deck1BpmDecrease,
    Deck1BaseBpmIncrease, Deck1BaseBpmDecrease, Deck1TempoReset,
    Deck1NudgeForward, Deck1NudgeBackward,
    Deck1OffsetIncrease, Deck1OffsetDecrease,
    Deck1JumpForward4b, Deck1JumpBackward4b,
    Deck1JumpForward8b, Deck1JumpBackward8b,
    Deck1JumpForward1bt, Deck1JumpBackward1bt,
    Deck1JumpForward4bt, Deck1JumpBackward4bt,
    Deck1Cue, Deck1CuePlay,
    // Deck 2
    Deck2PlayPause, Deck2OpenBrowser,
    Deck2LevelUp, Deck2LevelDown, Deck2LevelMax, Deck2LevelMin,
    Deck2FilterIncrease, Deck2FilterDecrease, Deck2FilterReset,
    Deck2BpmTap, Deck2MetronomeToggle, Deck2RedetectBpm,
    Deck2BpmIncrease, Deck2BpmDecrease,
    Deck2BaseBpmIncrease, Deck2BaseBpmDecrease, Deck2TempoReset,
    Deck2NudgeForward, Deck2NudgeBackward,
    Deck2OffsetIncrease, Deck2OffsetDecrease,
    Deck2JumpForward4b, Deck2JumpBackward4b,
    Deck2JumpForward8b, Deck2JumpBackward8b,
    Deck2JumpForward1bt, Deck2JumpBackward1bt,
    Deck2JumpForward4bt, Deck2JumpBackward4bt,
    Deck2Cue, Deck2CuePlay,
}

static ACTION_NAMES: &[(&str, Action)] = &[
    ("quit",              Action::Quit),
    ("help",              Action::Help),
    ("terminal_refresh",  Action::TerminalRefresh),
    ("zoom_in",           Action::ZoomIn),
    ("zoom_out",          Action::ZoomOut),
    ("height_increase",   Action::HeightIncrease),
    ("height_decrease",   Action::HeightDecrease),
    ("latency_increase",  Action::LatencyIncrease),
    ("latency_decrease",  Action::LatencyDecrease),
    ("waveform_style",    Action::WaveformStyle),
    ("palette_cycle",     Action::PaletteCycle),
    ("nudge_mode_toggle", Action::NudgeModeToggle),
    // Deck 1
    ("deck1_play_pause",        Action::Deck1PlayPause),
    ("deck1_open_browser",      Action::Deck1OpenBrowser),
    ("deck1_level_up",          Action::Deck1LevelUp),
    ("deck1_level_down",        Action::Deck1LevelDown),
    ("deck1_level_max",         Action::Deck1LevelMax),
    ("deck1_level_min",         Action::Deck1LevelMin),
    ("deck1_filter_increase",   Action::Deck1FilterIncrease),
    ("deck1_filter_decrease",   Action::Deck1FilterDecrease),
    ("deck1_filter_reset",      Action::Deck1FilterReset),
    ("deck1_bpm_tap",           Action::Deck1BpmTap),
    ("deck1_metronome",         Action::Deck1MetronomeToggle),
    ("deck1_redetect_bpm",      Action::Deck1RedetectBpm),
    ("deck1_bpm_increase",      Action::Deck1BpmIncrease),
    ("deck1_bpm_decrease",      Action::Deck1BpmDecrease),
    ("deck1_base_bpm_increase", Action::Deck1BaseBpmIncrease),
    ("deck1_base_bpm_decrease", Action::Deck1BaseBpmDecrease),
    ("deck1_tempo_reset",       Action::Deck1TempoReset),
    ("deck1_nudge_forward",     Action::Deck1NudgeForward),
    ("deck1_nudge_backward",    Action::Deck1NudgeBackward),
    ("deck1_offset_increase",   Action::Deck1OffsetIncrease),
    ("deck1_offset_decrease",   Action::Deck1OffsetDecrease),
    ("deck1_jump_forward_4b",   Action::Deck1JumpForward4b),
    ("deck1_jump_backward_4b",  Action::Deck1JumpBackward4b),
    ("deck1_jump_forward_8b",   Action::Deck1JumpForward8b),
    ("deck1_jump_backward_8b",  Action::Deck1JumpBackward8b),
    ("deck1_jump_forward_1bt",  Action::Deck1JumpForward1bt),
    ("deck1_jump_backward_1bt", Action::Deck1JumpBackward1bt),
    ("deck1_jump_forward_4bt",  Action::Deck1JumpForward4bt),
    ("deck1_jump_backward_4bt", Action::Deck1JumpBackward4bt),
    ("deck1_cue",               Action::Deck1Cue),
    ("deck1_cue_play",          Action::Deck1CuePlay),
    // Deck 2
    ("deck2_play_pause",        Action::Deck2PlayPause),
    ("deck2_open_browser",      Action::Deck2OpenBrowser),
    ("deck2_level_up",          Action::Deck2LevelUp),
    ("deck2_level_down",        Action::Deck2LevelDown),
    ("deck2_level_max",         Action::Deck2LevelMax),
    ("deck2_level_min",         Action::Deck2LevelMin),
    ("deck2_filter_increase",   Action::Deck2FilterIncrease),
    ("deck2_filter_decrease",   Action::Deck2FilterDecrease),
    ("deck2_filter_reset",      Action::Deck2FilterReset),
    ("deck2_bpm_tap",           Action::Deck2BpmTap),
    ("deck2_metronome",         Action::Deck2MetronomeToggle),
    ("deck2_redetect_bpm",      Action::Deck2RedetectBpm),
    ("deck2_bpm_increase",      Action::Deck2BpmIncrease),
    ("deck2_bpm_decrease",      Action::Deck2BpmDecrease),
    ("deck2_base_bpm_increase", Action::Deck2BaseBpmIncrease),
    ("deck2_base_bpm_decrease", Action::Deck2BaseBpmDecrease),
    ("deck2_tempo_reset",       Action::Deck2TempoReset),
    ("deck2_nudge_forward",     Action::Deck2NudgeForward),
    ("deck2_nudge_backward",    Action::Deck2NudgeBackward),
    ("deck2_offset_increase",   Action::Deck2OffsetIncrease),
    ("deck2_offset_decrease",   Action::Deck2OffsetDecrease),
    ("deck2_jump_forward_4b",   Action::Deck2JumpForward4b),
    ("deck2_jump_backward_4b",  Action::Deck2JumpBackward4b),
    ("deck2_jump_forward_8b",   Action::Deck2JumpForward8b),
    ("deck2_jump_backward_8b",  Action::Deck2JumpBackward8b),
    ("deck2_jump_forward_1bt",  Action::Deck2JumpForward1bt),
    ("deck2_jump_backward_1bt", Action::Deck2JumpBackward1bt),
    ("deck2_jump_forward_4bt",  Action::Deck2JumpForward4bt),
    ("deck2_jump_backward_4bt", Action::Deck2JumpBackward4bt),
    ("deck2_cue",               Action::Deck2Cue),
    ("deck2_cue_play",          Action::Deck2CuePlay),
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
    detail_height: usize,       // total rows per detail waveform (including 2-row tick area)
}

impl Default for DisplayConfig {
    fn default() -> Self { Self { playhead_position: 20, warning_threshold_secs: 30.0, detail_height: 6 } }
}

/// Finds or creates the config file and returns its text plus an optional notice.
fn resolve_config() -> (String, Option<String>) {
    // Check next to the binary first, then ~/.config/tj/config.toml, then auto-create.
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.toml")))
        .filter(|p| p.exists());
    if let Some(path) = adjacent {
        return (std::fs::read_to_string(&path).unwrap_or_default(), None);
    }
    let user_path = match home_dir() {
        Some(h) => h.join(".config/tj/config.toml"),
        None => return (DEFAULT_CONFIG.to_string(), None),
    };
    if user_path.exists() {
        (std::fs::read_to_string(&user_path).unwrap_or_default(), None)
    } else {
        // Auto-create from embedded default.
        if let Some(dir) = user_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let notice = if std::fs::write(&user_path, DEFAULT_CONFIG).is_ok() {
            Some(format!("config created: {}", user_path.display()))
        } else {
            None
        };
        (DEFAULT_CONFIG.to_string(), notice)
    }
}

fn load_config() -> (std::collections::HashMap<KeyBinding, Action>, DisplayConfig, Option<String>) {
    let (text, notice) = resolve_config();
    // Seed with defaults so any keys absent from the user config still work.
    let mut map = parse_keymap(DEFAULT_CONFIG, &mut std::collections::HashMap::new());
    let keymap = parse_keymap(&text, &mut map);
    let display = parse_display_config(&text);
    (keymap, display, notice)
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
    let detail_height = display
        .and_then(|v| v.get("detail_height"))
        .and_then(|v| v.as_integer())
        .unwrap_or(6)
        .max(3) as usize;
    DisplayConfig { playhead_position: pos, warning_threshold_secs, detail_height }
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
/// BPM = linear regression slope across all taps (beat index vs time), which converges
/// as taps accumulate — later taps add leverage and reduce variance.
/// Outlier taps (residual > half a beat period) are dropped before the final regression.
/// Offset = mean residual anchored to the first tap, avoiding phase drift from imprecise period.
fn compute_tap_bpm_offset(tap_times: &[f64]) -> (f32, i64) {
    let n = tap_times.len();
    if n < 2 { return (120.0, 0); }

    // First pass: regression over all taps to get a rough period for outlier detection.
    let beat_period = linear_regression_period(tap_times);
    if beat_period <= 0.0 { return (120.0, 0); }

    // Drop taps whose residual from the regression line exceeds half a beat period.
    let t0 = tap_times[0];
    let filtered: Vec<f64> = tap_times.iter().enumerate()
        .filter(|&(i, &t)| {
            let expected = t0 + i as f64 * beat_period;
            (t - expected).abs() < beat_period / 2.0
        })
        .map(|(_, &t)| t)
        .collect();
    let taps = if filtered.len() >= 2 { &filtered[..] } else { tap_times };

    // Second pass: refined regression on filtered taps.
    let beat_period = linear_regression_period(taps);
    if beat_period <= 0.0 { return (120.0, 0); }
    let bpm = (60.0 / beat_period) as f32;

    // Anchor residuals to the first tap so deltas are small.
    // Computing t % beat_period on large absolute positions causes phase drift when
    // beat_period is even slightly imprecise — error accumulates with distance from zero.
    let t0 = taps[0];
    let mean_residual = taps.iter()
        .map(|&t| { let d = t - t0; d - (d / beat_period).round() * beat_period })
        .sum::<f64>() / taps.len() as f64;
    let offset_secs = (t0 + mean_residual).rem_euclid(beat_period);
    let offset_ms = (offset_secs * 1000.0).round() as i64;
    (bpm.clamp(40.0, 240.0), offset_ms)
}

fn linear_regression_period(tap_times: &[f64]) -> f64 {
    let n = tap_times.len();
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean = tap_times.iter().sum::<f64>() / n as f64;
    let num: f64 = tap_times.iter().enumerate()
        .map(|(i, &y)| (i as f64 - x_mean) * (y - y_mean))
        .sum();
    let den: f64 = (0..n).map(|i| (i as f64 - x_mean).powi(2)).sum();
    if den <= 0.0 { return 0.0; }
    num / den
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

/// Compute 16 braille spectrum characters from mono samples at `pos`.
/// Uses the Goertzel algorithm on 32 log-spaced bins, 20 Hz – 20 kHz.
fn compute_spectrum(mono: &[f32], pos: usize, sample_rate: u32, filter_offset: i32) -> ([char; 16], [bool; 16]) {
    const N: usize = 4096;
    const LEFT_MASKS:  [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];
    const RIGHT_MASKS: [u8; 5] = [0x00, 0x80, 0xA0, 0xB0, 0xB8];

    // 32 log-spaced centre frequencies: 20 Hz … 20 kHz.
    let freqs: [f64; 32] = std::array::from_fn(|i| {
        20.0 * (1000.0_f64).powf(i as f64 / 31.0)
    });

    // Hann window coefficients — computed once and reused across all calls.
    static HANN: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    let hann = HANN.get_or_init(|| {
        (0..N)
            .map(|n| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * n as f64 / (N - 1) as f64).cos()) as f32)
            .collect()
    });

    // Pre-filter the window if a filter is active.
    let filtered: Vec<f32> = if filter_offset != 0 {
        let idx = (filter_offset.unsigned_abs() as usize - 1).min(15);
        let is_lpf = filter_offset < 0;
        let fc = if is_lpf { FILTER_CUTOFFS_HZ[idx] } else { FILTER_CUTOFFS_HZ[15 - idx] };
        let (b0, b1, b2, a1, a2) = butterworth_biquad(fc, sample_rate, is_lpf);
        let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        (0..N).map(|i| {
            let x = mono.get(pos + i).copied().unwrap_or(0.0);
            let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            x2 = x1; x1 = x; y2 = y1; y1 = y;
            y
        }).collect()
    } else {
        Vec::new()
    };

    let sr = sample_rate as f64;
    let mut heights = [0usize; 32];
    let mut raw_heights = [0.0f32; 32];

    for (k, &f) in freqs.iter().enumerate() {
        let coeff = 2.0 * (2.0 * std::f64::consts::PI * f / sr).cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for i in 0..N {
            let raw = if filter_offset != 0 {
                filtered[i]
            } else {
                mono.get(pos + i).copied().unwrap_or(0.0)
            };
            let sample = raw as f64 * hann[i] as f64;
            let s = sample + coeff * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        let magnitude = power.max(0.0).sqrt();
        let db = if magnitude > 0.0 { 20.0 * magnitude.log10() } else { 0.0 };
        // +3 dB/octave tilt to compensate for the ~1/f (pink noise) rolloff of typical music,
        // making treble bins as visible as bass bins with equal perceptual loudness.
        // 20 Hz → 0 dB boost; 20 kHz → +30 dB boost (~10 octaves × 3 dB).
        let tilt_db = (f / 20.0).log2() * 3.0;
        let raw = (db + tilt_db - 10.0) / 12.5;
        heights[k] = raw.round().clamp(0.0, 4.0) as usize;
        raw_heights[k] = raw as f32;
    }

    // Background active when raw energy exceeds 1/4 of the single-dot threshold (0.5).
    const BG_THRESH: f32 = 0.5 / 4.0;
    let chars: [char; 16] = std::array::from_fn(|c| {
        let l = heights[c * 2];
        let r = heights[c * 2 + 1];
        char::from_u32(0x2800 | LEFT_MASKS[l] as u32 | RIGHT_MASKS[r] as u32).unwrap_or(' ')
    });
    let has_bg: [bool; 16] = std::array::from_fn(|c| {
        raw_heights[c * 2] > BG_THRESH || raw_heights[c * 2 + 1] > BG_THRESH
    });
    (chars, has_bg)
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
fn bar_tick_cols(bpm: f64, offset_ms: i64, total_secs: f64, cols: usize) -> (Vec<usize>, Vec<f64>, u32) {
    if bpm <= 0.0 || total_secs <= 0.0 || cols == 0 {
        return (Vec::new(), Vec::new(), 4);
    }
    let beat_secs = 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let mut bars: u32 = 4;
    loop {
        let bar_period = bars as f64 * 4.0 * beat_secs; // bars × 4 beats/bar × secs/beat
        let n_start = (-offset_secs / bar_period).ceil() as i64;
        let mut result: Vec<(usize, f64)> = Vec::new();
        let mut t = offset_secs + n_start as f64 * bar_period;
        while t <= total_secs {
            let col = ((t / total_secs) * cols as f64).round() as usize;
            if col < cols {
                result.push((col, t.max(0.0)));
            }
            t += bar_period;
        }
        let min_gap = result.windows(2)
            .map(|w| w[1].0.saturating_sub(w[0].0))
            .min()
            .unwrap_or(usize::MAX);
        if min_gap >= 4 || bars >= 512 {
            let cols_vec = result.iter().map(|&(c, _)| c).collect();
            let times_vec = result.iter().map(|&(_, t)| t).collect();
            return (cols_vec, times_vec, bars);
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
            // Normal playback. Return silence past end so the source stays alive in the
            // player queue — allows seeking and replaying after end-of-track.
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            Some(self.samples.get(pos).copied().unwrap_or(0.0))
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

// ---------------------------------------------------------------------------
// Filter source — second-order Butterworth IIR biquad (LPF or HPF)
// ---------------------------------------------------------------------------

/// Log-spaced cutoff frequencies for filter offsets ±1..±16.
/// Index 0 = offset ±1 (near-flat), index 15 = offset ±16 (fully cut).
const FILTER_CUTOFFS_HZ: [f64; 16] = [
    18_000.0, 12_000.0,  8_000.0, 5_300.0,
     3_500.0,  2_350.0,  1_560.0, 1_040.0,
       690.0,    460.0,    306.0,   204.0,
       136.0,     90.0,     60.0,    40.0,
];

/// Compute normalised Butterworth biquad coefficients for a LPF or HPF.
/// Returns `(b0, b1, b2, a1, a2)` with a0 normalised to 1.
fn butterworth_biquad(fc: f64, sample_rate: u32, is_lpf: bool) -> (f32, f32, f32, f32, f32) {
    use std::f64::consts::PI;
    let w0 = 2.0 * PI * fc / sample_rate as f64;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / std::f64::consts::SQRT_2; // Q = 1/sqrt(2) → Butterworth
    let a0 = 1.0 + alpha;
    let (b0, b1, b2) = if is_lpf {
        ((1.0 - cos_w0) / 2.0, 1.0 - cos_w0, (1.0 - cos_w0) / 2.0)
    } else {
        ((1.0 + cos_w0) / 2.0, -(1.0 + cos_w0), (1.0 + cos_w0) / 2.0)
    };
    (
        (b0 / a0) as f32,
        (b1 / a0) as f32,
        (b2 / a0) as f32,
        (-2.0 * cos_w0 / a0) as f32,
        ((1.0 - alpha) / a0) as f32,
    )
}

struct FilterSource<S: Source<Item = f32>> {
    inner: S,
    filter_offset: Arc<AtomicI32>,
    filter_state_reset: Arc<AtomicBool>,
    /// Counts down from FADE_SAMPLES to 0 after a state reset; output is scaled
    /// by an ascending ramp so any IIR settling transient is inaudible.
    output_fade_remaining: u32,
    last_offset: i32,
    channels: u16,
    sample_rate: u32,
    // Per-channel biquad history
    x1: Vec<f32>, x2: Vec<f32>,
    y1: Vec<f32>, y2: Vec<f32>,
    // Normalised coefficients (a0 = 1)
    b0: f32, b1: f32, b2: f32, a1: f32, a2: f32,
    // Which channel slot we are about to emit
    ch_idx: usize,
}

impl<S: Source<Item = f32>> FilterSource<S> {
    fn new(inner: S, filter_offset: Arc<AtomicI32>, filter_state_reset: Arc<AtomicBool>) -> Self {
        let channels = inner.channels().get() as u16;
        let sample_rate = inner.sample_rate().get();
        let n = channels as usize;
        FilterSource {
            inner,
            filter_offset,
            filter_state_reset,
            output_fade_remaining: 0,
            last_offset: 0,
            channels,
            sample_rate,
            x1: vec![0.0; n], x2: vec![0.0; n],
            y1: vec![0.0; n], y2: vec![0.0; n],
            b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0,
            ch_idx: 0,
        }
    }

    fn recompute_coefficients(&mut self, offset: i32) {
        if offset == 0 { return; }
        let idx = (offset.unsigned_abs() as usize - 1).min(15);
        let is_lpf = offset < 0;
        let fc = if is_lpf {
            FILTER_CUTOFFS_HZ[idx]
        } else {
            FILTER_CUTOFFS_HZ[15 - idx]
        };
        let (b0, b1, b2, a1, a2) = butterworth_biquad(fc, self.sample_rate, is_lpf);
        self.b0 = b0; self.b1 = b1; self.b2 = b2;
        self.a1 = a1; self.a2 = a2;
        // State (x1, x2, y1, y2) is intentionally preserved so the filter
        // continues smoothly from its current history — zeroing it causes a click.
    }
}

impl<S: Source<Item = f32>> Iterator for FilterSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.filter_state_reset.swap(false, Ordering::Relaxed) {
            for v in self.x1.iter_mut().chain(&mut self.x2).chain(&mut self.y1).chain(&mut self.y2) {
                *v = 0.0;
            }
            self.last_offset = 0; // force recompute_coefficients on next sample
            self.output_fade_remaining = FADE_SAMPLES as u32;
        }
        let x = self.inner.next()?;
        let offset = self.filter_offset.load(Ordering::Relaxed);
        if offset != self.last_offset {
            self.last_offset = offset;
            self.recompute_coefficients(offset);
        }
        let ch = self.ch_idx;
        self.ch_idx = (ch + 1) % self.channels as usize;
        if offset == 0 {
            self.output_fade_remaining = 0; // cancel pending fade when filter is bypassed
            return Some(x);
        }
        let y = self.b0 * x + self.b1 * self.x1[ch] + self.b2 * self.x2[ch]
              - self.a1 * self.y1[ch] - self.a2 * self.y2[ch];
        self.x2[ch] = self.x1[ch]; self.x1[ch] = x;
        self.y2[ch] = self.y1[ch]; self.y1[ch] = y;
        if self.output_fade_remaining > 0 {
            let n = FADE_SAMPLES as u32 - self.output_fade_remaining;
            let t = n as f32 / FADE_SAMPLES as f32;
            self.output_fade_remaining -= 1;
            Some(y * t)
        } else {
            Some(y)
        }
    }
}

impl<S: Source<Item = f32>> Source for FilterSource<S> {
    fn current_span_len(&self) -> Option<usize> { self.inner.current_span_len() }
    fn channels(&self) -> NonZero<u16> { NonZero::new(self.channels).unwrap_or(NonZero::new(2).unwrap()) }
    fn sample_rate(&self) -> NonZero<u32> { NonZero::new(self.sample_rate).unwrap_or(NonZero::new(44100).unwrap()) }
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
    /// Find the quietest frame within ±10ms of `target_secs`, to minimise the fade-in transient.
    fn find_quiet_frame(&self, target_secs: f64) -> usize {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as i64;
        let window = self.sample_rate as i64 / 100; // 10ms in frames
        let search_start = (target_frame - window).max(0) as usize;
        let search_end = (target_frame + window).min(total_frames as i64) as usize;
        (search_start..=search_end)
            .min_by_key(|&f| {
                let base = f * frame_len;
                let amp: f32 = (0..frame_len)
                    .map(|c| self.samples.get(base + c).copied().unwrap_or(0.0).abs())
                    .sum();
                (amp * 1_000_000.0) as u64
            })
            .unwrap_or(target_frame.max(0) as usize)
    }

    fn seek_to(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let best_frame = self.find_quiet_frame(target_secs);
        let target_sample = (best_frame * frame_len).min(self.samples.len());

        // Store the target, then trigger fade-out. The audio thread applies the seek
        // when the fade-out completes and then fades back in.
        self.fade_len.store(FADE_SAMPLES, Ordering::SeqCst);
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-FADE_SAMPLES, Ordering::SeqCst);
    }

    /// Seek to `target_secs` directly, without a fade. Used when paused — the audio
    /// thread is not calling next(), so the fade-based seek would never execute.

    fn seek_direct(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let best_frame = self.find_quiet_frame(target_secs);
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
) -> EyreResult<(Vec<f32>, Vec<f32>, u32, u16)> {
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
        .ok_or_else(|| color_eyre::eyre::eyre!("no audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.ok_or_else(|| color_eyre::eyre::eyre!("track has no sample rate"))?;
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

fn read_track_name(path: &str) -> String {
    let fallback = || {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string()
    };
    let src = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return fallback(),
    };
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = match symphonia::default::get_probe().format(
        &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(_) => return fallback(),
    };
    let meta = probed.format.metadata();
    let tags = meta.current().map(|r| r.tags().to_vec()).unwrap_or_default();
    let find = |key: StandardTagKey| {
        tags.iter()
            .find(|t| t.std_key == Some(key))
            .map(|t| t.value.to_string())
    };
    let artist = find(StandardTagKey::Artist);
    let title = find(StandardTagKey::TrackTitle);
    match (artist, title) {
        (Some(a), Some(t)) => format!("{a} \u{2013} {t}"),
        (None, Some(t)) => t,
        _ => fallback(),
    }
}

// ---------------------------------------------------------------------------
// BPM cache
// ---------------------------------------------------------------------------

fn hash_mono(samples: &[f32]) -> String {
    let bytes = unsafe {
        std::slice::from_raw_parts(samples.as_ptr() as *const u8, samples.len() * 4)
    };
    blake3::Hasher::new().update(bytes).finalize().to_hex().to_string()
}

fn cache_path() -> std::path::PathBuf {
    home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".local/share/tj/cache.json")
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    bpm: f32,
    offset_ms: i64,
    /// Filename at time of first detection — informational only, not used as key.
    name: String,
    #[serde(default)]
    cue_sample: Option<usize>,
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

fn detect_bpm(samples: &[f32], sample_rate: u32) -> EyreResult<f32> {
    let result = analyze_audio(samples, sample_rate, AnalysisConfig::default())
        .map_err(|e| color_eyre::eyre::eyre!("stratum-dsp: {e:?}"))?;
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
        let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
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

        let mut dirs  = Vec::new();
        let mut audio = Vec::new();
        let mut other = Vec::new();
        for entry in raw {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                dirs.push(BrowserEntry { name, path, kind: EntryKind::Dir });
            } else if is_audio(&path) {
                audio.push(BrowserEntry { name, path, kind: EntryKind::Audio });
            } else {
                other.push(BrowserEntry { name, path, kind: EntryKind::Other });
            }
        }
        entries.extend(dirs);
        entries.extend(audio);
        entries.extend(other);

        // Start on the first selectable entry that isn't `..` so Enter navigates
        // into content rather than immediately going back up. `..` is reachable via Up.
        let cursor = entries
            .iter()
            .position(|e| Self::is_selectable(&e.kind) && e.name != "..")
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
                    KeyCode::Char('z') => return Ok((BrowserResult::ReturnToPlayer, state.cwd)),
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
