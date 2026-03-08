use std::io;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
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
const BEAT_UNITS: &[u32] = &[4, 8, 16, 32, 64, 128];
const DEFAULT_BEAT_UNIT_IDX: usize = 2; // 16 beats
const FADE_SAMPLES: i64 = 256; // ~5.8ms at 44100 Hz — fade-out then fade-in around each seek
const DETECT_MODES: &[&str] = &["auto", "fusion", "legacy"];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let start = match args.get(1) {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };

    // Set up terminal once — shared by browser and player.
    let setup = (|| -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        Terminal::new(CrosstermBackend::new(io::stdout()))
    })();
    let mut terminal = match setup {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Terminal error: {e}");
            std::process::exit(1);
        }
    };

    // If start is a directory (or CWD), run the browser first.
    let file_path = if start.is_file() {
        start
    } else {
        match run_browser(&mut terminal, start) {
            Ok(BrowserResult::Selected(p)) => p,
            Ok(BrowserResult::ReturnToPlayer) | Ok(BrowserResult::Quit) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(LeaveAlternateScreen);
                return;
            }
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(LeaveAlternateScreen);
                eprintln!("Browser error: {e}");
                std::process::exit(1);
            }
        }
    };

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            let _ = disable_raw_mode();
            let _ = io::stdout().execute(LeaveAlternateScreen);
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();
    let mut cache = Cache::load(cache_path());
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
                        let _ = io::stdout().execute(LeaveAlternateScreen);
                        return;
                    }
                }
            }
        };

        let (mono, stereo, sample_rate, channels) = match decode_result {
            Ok(v) => v,
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(LeaveAlternateScreen);
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
        let pending_target = Arc::new(AtomicUsize::new(usize::MAX));
        let seek_handle = SeekHandle {
            samples: Arc::clone(&samples),
            position: Arc::clone(&position),
            fade_remaining: Arc::clone(&fade_remaining),
            pending_target: Arc::clone(&pending_target),
            sample_rate,
            channels,
        };

        let player = Player::connect_new(&mixer);
        player.append(TrackingSource::new(
            samples, position, fade_remaining, pending_target, sample_rate, channels,
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
        ) {
            Ok(Some(path)) => { next_path = path; }
            Ok(None) => break,
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = io::stdout().execute(LeaveAlternateScreen);
                eprintln!("TUI error: {e}");
                std::process::exit(1);
            }
        }
    }

    let _ = disable_raw_mode();
    let _ = io::stdout().execute(LeaveAlternateScreen);
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
) -> io::Result<Option<std::path::PathBuf>> {
    let mut bpm: f32 = 120.0;
    let mut offset_ms: i64 = 0;
    let mut analysis_hash: Option<String> = None;
    let mut frame_count: usize = 0;
    // Smooth display position: advances via wall clock to avoid audio-buffer-burst jitter.
    let mut smooth_display_samp: f64 = 0.0;
    let mut last_render = Instant::now();
    let mut last_viewport_start: usize = 0;
    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let mut beat_unit_idx: usize = DEFAULT_BEAT_UNIT_IDX;
    let mut detect_mode: usize = 0;
    let mut detail_height: usize = 8;

    // Shared state for the background detail-braille thread.
    let detail_cols = Arc::new(AtomicUsize::new(0));
    let detail_rows = Arc::new(AtomicUsize::new(0));
    let detail_zoom_at = Arc::new(AtomicUsize::new(zoom_idx));
    let detail_braille_shared: Arc<Mutex<Arc<BrailleBuffer>>> =
        Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
    let live_mode_shared   = Arc::new(AtomicBool::new(false));
    let display_pos_shared = Arc::new(AtomicUsize::new(0));
    let mut live_mode = false;

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
        let braille_bg     = Arc::clone(&detail_braille_shared);
        let stop_bg        = Arc::clone(&stop_detail);
        let wf_bg          = Arc::clone(&waveform);

        let live_mode_bg   = Arc::clone(&live_mode_shared);
        let display_pos_bg = Arc::clone(&display_pos_shared);
        let sr_bg          = sample_rate;
        let ch_bg          = seek_handle.channels;

        thread::spawn(move || {
            let mut last_cols            = 0usize;
            let mut last_rows            = 0usize;
            let mut last_zoom            = usize::MAX;
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
                let live         = live_mode_bg.load(Ordering::Relaxed);
                // Always use the smooth display position as the buffer centre — even in buffer mode.
                // Using the raw audio position (pos_bg) causes premature recomputes and off-centre
                // buffers whenever rodio bursts the audio position forward.
                let pos_samp     = display_pos_bg.load(Ordering::Relaxed) / ch_bg as usize;
                let col_samp     = ((zoom_secs * sr_bg as f32) as usize / cols).max(1);

                // Recompute when dimensions/zoom change or the viewport approaches the buffer edge.
                // In live mode, always recompute.
                let drift_cols = if last_samples_per_col > 0 {
                    let drift = pos_samp as i64 - last_anchor_sample as i64;
                    drift.unsigned_abs() as usize / last_samples_per_col
                } else {
                    usize::MAX // force initial compute
                };

                let must_recompute = live
                    || cols != last_cols || rows != last_rows || zoom != last_zoom || drift_cols >= cols * 3 / 4;

                if must_recompute {
                    let buf_cols = if live { cols * 2 } else { cols * 3 };
                    // Align anchor to the column grid so every buffer at this zoom level shares
                    // the same column boundaries — overlapping columns are byte-for-byte identical,
                    // making the buffer handoff visually seamless.
                    let anchor = (pos_samp / col_samp) * col_samp;
                    let mono   = &wf_bg.mono;

                    let peaks: Vec<(f32, f32)> = (0..buf_cols).map(|c| {
                        let offset     = c as i64 - (buf_cols / 2) as i64;
                        let samp_start = (anchor as i64 + offset * col_samp as i64).max(0) as usize;
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
                        grid: render_braille(&peaks, rows, buf_cols),
                        buf_cols,
                        anchor_sample:   anchor,
                        samples_per_col: col_samp,
                    });
                    last_cols            = cols;
                    last_rows            = rows;
                    last_zoom            = zoom;
                    last_anchor_sample   = anchor;
                    last_samples_per_col = col_samp;
                }

                thread::sleep(Duration::from_millis(if live { 4 } else { 8 }));
            }
        });
    }

    loop {
        frame_count += 1;
        if let Ok((hash, new_bpm, new_offset)) = bpm_rx.try_recv() {
            bpm = new_bpm;
            offset_ms = new_offset;
            cache.set(hash.clone(), CacheEntry { bpm, offset_ms, name: filename.to_string() });
            cache.save();
            analysis_hash = Some(hash);
        }

        let now = Instant::now();
        let dc = detail_cols.load(Ordering::Relaxed);
        let zoom_secs = ZOOM_LEVELS[zoom_idx];
        let col_secs = if dc > 0 { zoom_secs as f64 / dc as f64 } else { 0.033 };
        let elapsed = now.duration_since(last_render).as_secs_f64()
            .min(col_secs * 0.75); // cap at 1.5× a dot-column (= 0.75× a full column)
        last_render = now;

        // Real audio position — used for beat flash, time display, overview playhead.
        let pos_raw  = seek_handle.position.load(Ordering::Relaxed);
        let pos_samp = pos_raw / seek_handle.channels as usize;
        let pos      = Duration::from_secs_f64(pos_samp as f64 / sample_rate as f64);

        // Smooth display position — advances via wall clock to avoid audio-buffer-burst jitter.
        // Large drift (seek / startup) snaps immediately. Small drift correction rate is 1.0
        // (also snaps) so any firing is visually obvious — for empirical observation only.
        if !player.is_paused() {
            smooth_display_samp += elapsed * sample_rate as f64;
        }
        let drift = smooth_display_samp - pos_samp as f64;
        if drift.abs() > sample_rate as f64 * 0.5 {
            // Large drift (seek / startup) — snap immediately.
            smooth_display_samp = pos_samp as f64;
        }
        let display_pos_samp = smooth_display_samp as usize;
        display_pos_shared.store(display_pos_samp * seek_handle.channels as usize, Ordering::Relaxed);
        live_mode_shared.store(live_mode, Ordering::Relaxed);

        // Beat-derived values — recomputed each frame so they react to bpm changes instantly.
        let beat_period = Duration::from_secs_f64(60.0 / bpm as f64);
        let flash_window = beat_period.mul_f64(0.15);

        // Beat flash
        let pos_ns = pos.as_nanos() as i128 + offset_ms as i128 * 1_000_000;
        let phase = pos_ns.rem_euclid(beat_period.as_nanos() as i128);
        let beat_on = phase < flash_window.as_nanos() as i128;

        let analysing = analysis_hash.is_none();

        let fmt_dur = |d: Duration| {
            let s = d.as_secs();
            format!("{:02}:{:02}", s / 60, s % 60)
        };
        let time_str = format!("{} / {}", fmt_dur(pos), fmt_dur(total_duration));
        let status = if player.is_paused() { "Paused" } else { "Playing" };
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
                    Constraint::Length(1), // BPM + offset
                    Constraint::Length(5), // overview waveform
                    Constraint::Min(0),    // detail waveform + blank space
                    Constraint::Length(1), // beat indicator
                    Constraint::Length(1), // status + time
                    Constraint::Length(1), // key hints
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

            // BPM + offset
            let bpm_line = if analysing {
                format!("BPM: --- [analysing {}]   unit: {} beats",
                    SPINNER[frame_count % SPINNER.len()], BEAT_UNITS[beat_unit_idx])
            } else {
                format!("BPM: {bpm:.0} [{}]   offset: {:+}ms   unit: {} beats",
                    DETECT_MODES[detect_mode], offset_ms, BEAT_UNITS[beat_unit_idx])
            };
            frame.render_widget(Paragraph::new(bpm_line), chunks[0]);

            // Overview waveform — Braille rendered fresh each frame (O(cols×rows), negligible).
            let ow = chunks[1].width as usize;
            let oh = chunks[1].height as usize;
            let total_peaks = waveform.peaks.len();
            let playhead_frac = if total_duration.is_zero() {
                0.0
            } else {
                pos.as_secs_f64() / total_duration.as_secs_f64()
            };
            let playhead_col = ((playhead_frac * ow as f64) as usize).min(ow.saturating_sub(1));

            let ov_peaks: Vec<(f32, f32)> = (0..ow)
                .map(|col| {
                    let idx = (col * total_peaks / ow.max(1)).min(total_peaks.saturating_sub(1));
                    waveform.peaks[idx]
                })
                .collect();
            let ov_braille = render_braille(&ov_peaks, oh, ow);
            let bar_cols: Vec<usize> = if !analysing {
                bar_tick_cols(bpm as f64, offset_ms, total_duration.as_secs_f64(), ow)
            } else {
                Vec::new()
            };

            let ov_lines: Vec<Line<'static>> = ov_braille
                .into_iter()
                .map(|row| {
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut run = String::new();
                    let mut run_color = Color::Reset;
                    for (c, byte) in row.into_iter().enumerate() {
                        // Replicate Canvas z-order: tick drawn first, waveform on top.
                        // Tick is visible only where the waveform cell is empty (byte == 0).
                        let (color, ch) = if c == playhead_col {
                            (Color::White, '\u{28FF}') // ⣿ solid playhead
                        } else if bar_cols.contains(&c) && byte == 0 {
                            (Color::DarkGray, '\u{28FF}') // ⣿ tick visible in gap
                        } else {
                            (Color::Green, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
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
            detail_rows.store(dh, Ordering::Relaxed);
            let buf = Arc::clone(&*detail_braille_shared.lock().unwrap());
            let centre_col = dw / 2;

            // Compute the column offset into the buffer that places the playhead at centre.
            // Uses smooth_display_samp (wall-clock based) to avoid audio-buffer-burst jitter.
            // Also derive the quantised viewport centre in seconds for beat_line_cols, so that
            // ticks are anchored to the same column grid as the waveform (avoids ±1-col jitter).
            let mut viewport_centre_secs = display_pos_samp as f64 / sample_rate as f64;
            let mut sub_col = false;
            let viewport_start: Option<usize> = if buf.buf_cols >= dw && buf.samples_per_col > 0 {
                let delta = display_pos_samp as i64 - buf.anchor_sample as i64;
                // Track at half-column (dot-column) resolution for sub-column scrolling.
                let half_col_samp = buf.samples_per_col as f64 / 2.0;
                let delta_half = (delta as f64 / half_col_samp).round() as i64;
                sub_col = delta_half % 2 != 0;
                let delta_cols = delta_half / 2;
                let vs = buf.buf_cols as i64 / 2 + delta_cols - dw as i64 / 2;
                // Snap beat_line_cols centre to the same quantised position as the viewport,
                // offset by half a column when sub_col so ticks stay aligned with the waveform.
                let sub_col_offset = if sub_col { buf.samples_per_col as f64 / 2.0 } else { 0.0 };
                viewport_centre_secs = ((buf.anchor_sample as i64 + delta_cols * buf.samples_per_col as i64)
                    .max(0) as f64 + sub_col_offset) / sample_rate as f64;
                // Need dw+1 columns when sub_col to supply the extra byte for the shift.
                let need = if sub_col { dw + 1 } else { dw };
                if vs >= 0 && (vs as usize) + need <= buf.buf_cols {
                    let v = vs as usize;
                    last_viewport_start = v;
                    Some(v)
                } else {
                    // Buffer not yet ready (new buffer being computed) — reuse last valid frame.
                    if buf.buf_cols >= dw && last_viewport_start + need <= buf.buf_cols {
                        Some(last_viewport_start)
                    } else {
                        None // first frame or seek — show blank
                    }
                }
            } else {
                None
            };

            let beat_cols: Vec<usize> = if !analysing {
                beat_line_cols(bpm as f64, offset_ms, viewport_centre_secs, zoom_secs as f64, dw)
            } else {
                Vec::new()
            };

            let detail_lines: Vec<Line<'static>> = (0..dh)
                .map(|r| {
                    // When sub_col, shift each character by one dot-column using the next byte.
                    let shifted: Option<Vec<u8>> = if sub_col {
                        viewport_start.and_then(|vs| {
                            buf.grid.get(r).map(|row| {
                                (0..dw).map(|c| shift_braille_half(row[vs + c], row[vs + c + 1])).collect()
                            })
                        })
                    } else {
                        None
                    };
                    let row_slice: Option<&[u8]> = if sub_col {
                        shifted.as_deref()
                    } else {
                        viewport_start.and_then(|vs| buf.grid.get(r).map(|row| &row[vs..vs + dw]))
                    };
                    let row = match row_slice {
                        None => return Line::from(Span::raw("\u{2800}".repeat(dw))),
                        Some(s) => s,
                    };
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut run = String::new();
                    let mut run_color = Color::Reset;
                    for (c, &byte) in row.iter().enumerate() {
                        // Replicate Canvas z-order: tick drawn first, waveform on top.
                        // Tick is visible only where the waveform cell is empty (byte == 0).
                        let (color, ch) = if c == centre_col {
                            (Color::White, '\u{28FF}') // ⣿ solid centre line
                        } else if beat_cols.contains(&c) && byte == 0 {
                            (Color::DarkGray, '\u{28FF}') // ⣿ beat tick visible in gap
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

            // Beat indicator (single line)
            let (label, style) = if beat_on && !analysing {
                (
                    "  ██  BEAT  ██  ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ··  beat  ··  ", Style::default().fg(Color::DarkGray))
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(label, style))),
                chunks[3],
            );

            // Status + time
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("[{status}]  "), Style::default().fg(Color::Cyan)),
                    Span::raw(time_str),
                    Span::styled(
                        format!("   zoom: {}s", zoom_secs),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                chunks[4],
            );

            // Key hints
            frame.render_widget(
                Paragraph::new(format!("Space: play/pause   [/]: beat jump   1-6: unit   +/-: offset   z/Z: zoom   {{/}}: height({})   m: mode({})   h/H: bpm½/×2   r: re-detect   b: browser   q: quit",
                    detail_height, if live_mode { "live" } else { "buf" })
                )
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[5],
            );
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
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        player.stop();
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                                cache.save();
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Char('b') => {
                        match run_browser(terminal, file_dir.to_path_buf())? {
                            BrowserResult::ReturnToPlayer => {}
                            BrowserResult::Selected(path) => {
                                player.stop();
                                if let Some(ref hash) = analysis_hash {
                                    if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                        cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                                        cache.save();
                                    }
                                }
                                return Ok(Some(path));
                            }
                            BrowserResult::Quit => {
                                player.stop();
                                if let Some(ref hash) = analysis_hash {
                                    if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                        cache.set(hash.clone(), CacheEntry { offset_ms, ..entry });
                                        cache.save();
                                    }
                                }
                                return Ok(None);
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        if player.is_paused() { player.play(); } else { player.pause(); }
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => offset_ms += 10,
                    KeyCode::Char('-') | KeyCode::Char('_') => offset_ms -= 10,
                    KeyCode::Char('z') => {
                        if zoom_idx > 0 { zoom_idx -= 1; }
                    }
                    KeyCode::Char('Z') => {
                        if zoom_idx + 1 < ZOOM_LEVELS.len() { zoom_idx += 1; }
                    }
                    KeyCode::Char('m') => {
                        live_mode = !live_mode;
                    }
                    KeyCode::Char('{') => {
                        if detail_height > 1 { detail_height -= 1; }
                    }
                    KeyCode::Char('}') => {
                        detail_height += 1; // clamped below at render time by ratatui
                    }
                    KeyCode::Char('h') => {
                        bpm = (bpm * 0.5).max(40.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    KeyCode::Char('H') => {
                        bpm = (bpm * 2.0).min(240.0);
                        if let Some(ref hash) = analysis_hash {
                            if let Some(entry) = cache.get(hash.as_str()).cloned() {
                                cache.set(hash.clone(), CacheEntry { bpm, offset_ms, ..entry });
                                cache.save();
                            }
                        }
                    }
                    KeyCode::Char('r') => {
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
                        }
                    }
                    KeyCode::Char(c @ '1'..='6') => {
                        beat_unit_idx = (c as usize - '1' as usize).min(BEAT_UNITS.len() - 1);
                    }
                    KeyCode::Char('[') => {
                        let jump = BEAT_UNITS[beat_unit_idx] as f64 * 60.0 / bpm as f64;
                        let target = seek_handle.current_pos().as_secs_f64() - jump;
                        seek_handle.seek_to(target.max(0.0));
                    }
                    KeyCode::Char(']') => {
                        let jump = BEAT_UNITS[beat_unit_idx] as f64 * 60.0 / bpm as f64;
                        let target = seek_handle.current_pos().as_secs_f64() + jump;
                        let max = total_duration.as_secs_f64();
                        if target < max {
                            seek_handle.seek_to(target);
                        }
                    }
                    _ => {}
                }
            }
        }

        if player.empty() {
            return Ok(None);
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
/// Takes the right dot-column of `a` (bits 3,4,5,7) as the new left column (bits 0,1,2,6)
/// and the left dot-column of `b` (bits 0,1,2,6) as the new right column (bits 3,4,5,7).
fn shift_braille_half(a: u8, b: u8) -> u8 {
    let left  = ((a >> 3) & 0x07) | ((a >> 1) & 0x40);
    let right = ((b & 0x07) << 3) | ((b & 0x40) << 1);
    left | right
}

fn render_braille(peaks: &[(f32, f32)], rows: usize, cols: usize) -> Vec<Vec<u8>> {
    // Bit mask for left+right dots at each of the 4 dot-rows within a Braille cell.
    // Layout: dot1(bit0)/dot4(bit3), dot2(bit1)/dot5(bit4), dot3(bit2)/dot6(bit5), dot7(bit6)/dot8(bit7)
    const DOT_BITS: [u8; 4] = [0x09, 0x12, 0x24, 0xC0];

    let mut grid = vec![vec![0u8; cols]; rows];
    if rows == 0 || cols == 0 {
        return grid;
    }
    let total_dots = rows * 4;

    for (c, &(min_val, max_val)) in peaks.iter().take(cols).enumerate() {
        let clamped_max = max_val.min(1.0);
        let clamped_min = min_val.max(-1.0);
        if clamped_min > clamped_max {
            continue;
        }
        // Map y ∈ [-1, 1] → dot row ∈ [0, total_dots); y=1 is top (row 0).
        let top_dot = ((1.0 - clamped_max) / 2.0 * total_dots as f32) as usize;
        let bot_dot = (((1.0 - clamped_min) / 2.0 * total_dots as f32) as usize)
            .min(total_dots - 1);
        for d in top_dot..=bot_dot {
            let br = d / 4;
            let dr = d % 4;
            if br < rows {
                grid[br][c] |= DOT_BITS[dr];
            }
        }
    }
    grid
}

/// Return the column indices of beat lines within the detail view window.
///
/// Replaces `draw_beat_lines` — callers colour these columns instead of drawing Canvas lines.
fn beat_line_cols(
    bpm: f64,
    offset_ms: i64,
    pos_secs: f64,
    zoom_secs: f64,
    cols: usize,
) -> Vec<usize> {
    let mut result = Vec::new();
    if bpm <= 0.0 || zoom_secs <= 0.0 || cols == 0 {
        return result;
    }
    let beat_period = 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let window_start = pos_secs - zoom_secs / 2.0;
    let window_end = pos_secs + zoom_secs / 2.0;
    let n_start = ((window_start - offset_secs) / beat_period).ceil() as i64;
    let mut t = offset_secs + n_start as f64 * beat_period;
    while t <= window_end {
        if t >= window_start {
            let col = ((t - window_start) / zoom_secs * cols as f64).round() as usize;
            if col < cols {
                result.push(col);
            }
        }
        t += beat_period;
    }
    result
}

/// Return the column indices of bar-tick lines within the overview.
///
/// Replaces `draw_bar_ticks` — callers colour these columns instead of drawing Canvas lines.
fn bar_tick_cols(bpm: f64, offset_ms: i64, total_secs: f64, cols: usize) -> Vec<usize> {
    let mut result = Vec::new();
    if bpm <= 0.0 || total_secs <= 0.0 || cols == 0 {
        return result;
    }
    let bar_period = 16.0 * 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let n_start = (-offset_secs / bar_period).ceil() as i64;
    let mut t = offset_secs + n_start as f64 * bar_period;
    while t <= total_secs {
        let col = ((t / total_secs) * cols as f64).round() as usize;
        if col < cols {
            result.push(col);
        }
        t += bar_period;
    }
    result
}

// ---------------------------------------------------------------------------
// Waveform data
// ---------------------------------------------------------------------------

struct WaveformData {
    /// Full-track peak envelope at OVERVIEW_RESOLUTION buckets.
    peaks: Vec<(f32, f32)>,
    /// Raw mono samples for detail view rendering.
    mono: Arc<Vec<f32>>,
}

impl WaveformData {
    fn compute(mono: Arc<Vec<f32>>, _sample_rate: u32) -> Self {
        let n = mono.len();
        let chunk_size = (n / OVERVIEW_RESOLUTION).max(1);
        let peaks = mono
            .chunks(chunk_size)
            .map(|chunk| {
                let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                (min.max(-1.0), max.min(1.0))
            })
            .collect();
        Self { peaks, mono }
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
        pending_target: Arc<AtomicUsize>,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        Self { samples, position, fade_remaining, pending_target, sample_rate, channels }
    }
}

impl Iterator for TrackingSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let fade = self.fade_remaining.load(Ordering::Relaxed);

        if fade < 0 {
            // Fading out: read current position, apply descending envelope.
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            let raw = self.samples.get(pos).copied().unwrap_or(0.0);
            let t = (-fade) as f32 / FADE_SAMPLES as f32;
            let new_fade = self.fade_remaining.fetch_add(1, Ordering::Relaxed) + 1;
            if new_fade == 0 {
                // Fade-out complete — apply pending seek then start fade-in.
                let target = self.pending_target.swap(usize::MAX, Ordering::SeqCst);
                if target != usize::MAX {
                    self.position.store(target, Ordering::SeqCst);
                }
                self.fade_remaining.store(FADE_SAMPLES, Ordering::Relaxed);
            }
            Some(raw * t)
        } else if fade > 0 {
            // Fading in: read new position, apply ascending envelope.
            let pos = self.position.fetch_add(1, Ordering::Relaxed);
            let raw = self.samples.get(pos).copied().unwrap_or(0.0);
            let t = (FADE_SAMPLES - fade) as f32 / FADE_SAMPLES as f32;
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
    fn current_span_len(&self) -> Option<usize> { None }
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
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-FADE_SAMPLES, Ordering::SeqCst);
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

struct Cache {
    path: std::path::PathBuf,
    entries: std::collections::HashMap<String, CacheEntry>,
}

impl Cache {
    fn load(path: std::path::PathBuf) -> Self {
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, entries }
    }

    fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.entries.get(hash)
    }

    fn set(&mut self, hash: String, entry: CacheEntry) {
        self.entries.insert(hash, entry);
    }

    fn entries_snapshot(&self) -> std::collections::HashMap<String, CacheEntry> {
        self.entries.clone()
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("tmp");
        if let Ok(text) = serde_json::to_string_pretty(&self.entries) {
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
) -> io::Result<BrowserResult> {
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
                                EntryKind::Audio => return Ok(BrowserResult::Selected(entry.path.clone())),
                                EntryKind::Other => {}
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Left => {
                        if let Some(parent) = state.cwd.parent().map(|p| p.to_path_buf()) {
                            state = BrowserState::new(parent)?;
                        }
                    }
                    KeyCode::Char('q') => return Ok(BrowserResult::Quit),
                    KeyCode::Esc => return Ok(BrowserResult::ReturnToPlayer),
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
