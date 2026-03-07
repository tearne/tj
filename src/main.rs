use std::io;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
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

        let _ = terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(
                Paragraph::new(format!("Loading {}…", path_str))
                    .style(Style::default().fg(Color::DarkGray)),
                area,
            );
        });

        let (mono, stereo, sample_rate, channels) = match decode_audio(&path_str) {
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

        let hash = hash_mono(&mono);

        let (bpm, initial_offset_ms) = if let Some(entry) = cache.get(&hash) {
            (entry.bpm, entry.offset_ms)
        } else {
            let bpm = match detect_bpm(&mono, sample_rate) {
                Ok(b) => b.round(),
                Err(e) => {
                    let _ = disable_raw_mode();
                    let _ = io::stdout().execute(LeaveAlternateScreen);
                    eprintln!("BPM error: {e}");
                    std::process::exit(1);
                }
            };
            cache.set(hash.clone(), CacheEntry { bpm, offset_ms: 0, name: filename.clone() });
            cache.save();
            (bpm, 0)
        };

        let total_duration = Duration::from_secs(mono.len() as u64 / sample_rate as u64);
        let waveform = WaveformData::compute(mono, sample_rate);

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

        match tui_loop(
            &mut terminal,
            &filename,
            &file_dir,
            bpm,
            initial_offset_ms,
            total_duration,
            &waveform,
            &player,
            &seek_handle,
            &hash,
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
    bpm: f32,
    initial_offset_ms: i64,
    total_duration: Duration,
    waveform: &WaveformData,
    player: &Player,
    seek_handle: &SeekHandle,
    hash: &str,
    cache: &mut Cache,
) -> io::Result<Option<std::path::PathBuf>> {
    let beat_period = Duration::from_secs_f64(60.0 / bpm as f64);
    let flash_window = beat_period.mul_f64(0.15);
    let mut offset_ms: i64 = initial_offset_ms;
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let mut beat_unit_idx: usize = DEFAULT_BEAT_UNIT_IDX;

    loop {
        let pos = seek_handle.current_pos();

        // Beat flash
        let pos_ns = pos.as_nanos() as i128 + offset_ms as i128 * 1_000_000;
        let phase = pos_ns.rem_euclid(beat_period.as_nanos() as i128);
        let beat_on = phase < flash_window.as_nanos() as i128;

        let fmt_dur = |d: Duration| {
            let s = d.as_secs();
            format!("{:02}:{:02}", s / 60, s % 60)
        };
        let time_str = format!("{} / {}", fmt_dur(pos), fmt_dur(total_duration));
        let status = if player.is_paused() { "Paused" } else { "Playing" };
        let zoom_secs = ZOOM_LEVELS[zoom_idx];

        terminal.draw(|frame| {
            let area = frame.area();

            let outer = Block::default()
                .title(format!(" tj — {filename} "))
                .borders(Borders::ALL);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // BPM + offset
                    Constraint::Length(5), // overview waveform
                    Constraint::Min(6),    // detail waveform
                    Constraint::Length(1), // beat indicator
                    Constraint::Length(1), // status + time
                    Constraint::Length(1), // key hints
                ])
                .split(inner);

            // BPM + offset
            frame.render_widget(
                Paragraph::new(format!(
                    "BPM: {bpm:.0}   offset: {:+}ms   unit: {} beats",
                    offset_ms, BEAT_UNITS[beat_unit_idx]
                )),
                chunks[0],
            );

            // Overview waveform
            let ow = chunks[1].width as usize;
            let total_peaks = waveform.peaks.len();
            let playhead_frac = if total_duration.is_zero() {
                0.0
            } else {
                pos.as_secs_f64() / total_duration.as_secs_f64()
            };
            let playhead_col = (playhead_frac * ow as f64).min(ow as f64 - 1.0);

            frame.render_widget(
                Canvas::default()
                    .marker(Marker::Braille)
                    .x_bounds([0.0, ow as f64])
                    .y_bounds([-1.0, 1.0])
                    .paint(move |ctx| {
                        draw_bar_ticks(ctx, bpm as f64, offset_ms, total_duration.as_secs_f64(), ow as f64);
                        for col in 0..ow {
                            let idx = (col * total_peaks / ow.max(1)).min(total_peaks - 1);
                            let (min, max) = waveform.peaks[idx];
                            ctx.draw(&CanvasLine {
                                x1: col as f64, y1: min as f64,
                                x2: col as f64, y2: max as f64,
                                color: Color::Green,
                            });
                        }
                        ctx.draw(&CanvasLine {
                            x1: playhead_col, y1: -1.0,
                            x2: playhead_col, y2: 1.0,
                            color: Color::White,
                        });
                    }),
                chunks[1],
            );

            // Detail waveform
            let dw = chunks[2].width as usize;
            let detail_peaks = waveform.detail_peaks(pos, zoom_secs, dw);
            let center_col = dw as f64 / 2.0;

            frame.render_widget(
                Canvas::default()
                    .marker(Marker::Braille)
                    .x_bounds([0.0, dw as f64])
                    .y_bounds([-1.0, 1.0])
                    .paint(move |ctx| {
                        draw_beat_lines(ctx, bpm as f64, offset_ms, pos.as_secs_f64(), zoom_secs as f64, dw as f64);
                        for (col, (min, max)) in detail_peaks.iter().enumerate() {
                            ctx.draw(&CanvasLine {
                                x1: col as f64, y1: *min as f64,
                                x2: col as f64, y2: *max as f64,
                                color: Color::Cyan,
                            });
                        }
                        ctx.draw(&CanvasLine {
                            x1: center_col, y1: -1.0,
                            x2: center_col, y2: 1.0,
                            color: Color::White,
                        });
                    }),
                chunks[2],
            );

            // Beat indicator (single line)
            let (label, style) = if beat_on {
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
                Paragraph::new("Space: play/pause   [/]: beat jump   1-6: unit   +/-: offset   z/Z: zoom   b: browser   q: quit")
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[5],
            );
        })?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        player.stop();
                        if let Some(entry) = cache.get(hash).cloned() {
                            cache.set(hash.to_string(), CacheEntry { offset_ms, ..entry });
                            cache.save();
                        }
                        return Ok(None);
                    }
                    KeyCode::Char('b') => {
                        match run_browser(terminal, file_dir.to_path_buf())? {
                            BrowserResult::ReturnToPlayer => {}
                            BrowserResult::Selected(path) => {
                                player.stop();
                                if let Some(entry) = cache.get(hash).cloned() {
                                    cache.set(hash.to_string(), CacheEntry { offset_ms, ..entry });
                                    cache.save();
                                }
                                return Ok(Some(path));
                            }
                            BrowserResult::Quit => {
                                player.stop();
                                if let Some(entry) = cache.get(hash).cloned() {
                                    cache.set(hash.to_string(), CacheEntry { offset_ms, ..entry });
                                    cache.save();
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

/// Draw bar ticks (every 4 beats) along the bottom of the overview canvas.
fn draw_bar_ticks(ctx: &mut Context<'_>, bpm: f64, offset_ms: i64, total_secs: f64, width: f64) {
    if bpm <= 0.0 || total_secs <= 0.0 {
        return;
    }
    let bar_period = 16.0 * 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let n_start = (-offset_secs / bar_period).ceil() as i64;
    let mut t = offset_secs + n_start as f64 * bar_period;
    while t <= total_secs {
        let x = ((t / total_secs) * width).round();
        ctx.draw(&CanvasLine { x1: x, y1: -1.0, x2: x, y2: 1.0, color: Color::DarkGray });
        t += bar_period;
    }
}

/// Draw full-height beat lines on the detail canvas, drawn before the waveform so the
/// waveform paints over them — markers are only visible in the gaps between waveform peaks.
fn draw_beat_lines(ctx: &mut Context<'_>, bpm: f64, offset_ms: i64, pos_secs: f64, zoom_secs: f64, width: f64) {
    if bpm <= 0.0 || zoom_secs <= 0.0 {
        return;
    }
    let beat_period = 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let window_start = pos_secs - zoom_secs / 2.0;
    let window_end = pos_secs + zoom_secs / 2.0;
    let n_start = ((window_start - offset_secs) / beat_period).ceil() as i64;
    let mut t = offset_secs + n_start as f64 * beat_period;
    while t <= window_end {
        if t >= window_start {
            let x = ((t - window_start) / zoom_secs * width).round();
            ctx.draw(&CanvasLine { x1: x, y1: -1.0, x2: x, y2: 1.0, color: Color::DarkGray });
        }
        t += beat_period;
    }
}

// ---------------------------------------------------------------------------
// Waveform data
// ---------------------------------------------------------------------------

struct WaveformData {
    /// Full-track peak envelope at OVERVIEW_RESOLUTION buckets.
    peaks: Vec<(f32, f32)>,
    /// Raw mono samples for detail view rendering.
    mono: Vec<f32>,
    sample_rate: u32,
}

impl WaveformData {
    fn compute(mono: Vec<f32>, sample_rate: u32) -> Self {
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
        Self { peaks, mono, sample_rate }
    }

    /// Compute peak envelope for the detail view window centred on `pos`.
    /// Returns `cols` (min, max) pairs.
    fn detail_peaks(&self, pos: Duration, window_secs: f32, cols: usize) -> Vec<(f32, f32)> {
        if cols == 0 {
            return vec![];
        }
        let center = (pos.as_secs_f32() * self.sample_rate as f32) as usize;
        let half = (window_secs * self.sample_rate as f32 / 2.0) as usize;
        let start = center.saturating_sub(half);
        let end = (center + half).min(self.mono.len());
        let window = &self.mono[start..end];

        if window.is_empty() {
            return vec![(0.0, 0.0); cols];
        }

        let chunk_size = (window.len() / cols).max(1);
        let mut result: Vec<(f32, f32)> = window
            .chunks(chunk_size)
            .map(|chunk| {
                let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                (min.max(-1.0), max.min(1.0))
            })
            .collect();
        result.resize(cols, (0.0, 0.0));
        result
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
fn decode_audio(path: &str) -> Result<(Vec<f32>, Vec<f32>, u32, u16), Box<dyn std::error::Error>> {
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
