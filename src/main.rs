use std::io;
use std::num::NonZero;
use std::path::Path;
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
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, Paragraph};
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: tj <file>");
        std::process::exit(1);
    }
    let path = &args[1];

    eprintln!("Decoding {}...", path);
    let (mono, stereo, sample_rate, channels) = match decode_audio(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Decode error: {e}");
            std::process::exit(1);
        }
    };

    let sidecar_path = format!("{}.tj", path);
    let sidecar = Sidecar::load(&sidecar_path);

    let (bpm, initial_offset_ms) = if let Some(s) = sidecar {
        eprintln!("Loaded BPM from sidecar: {}", s.bpm);
        (s.bpm, s.offset_ms)
    } else {
        eprintln!("Detecting BPM...");
        let bpm = match detect_bpm(&mono, sample_rate) {
            Ok(b) => b.round(),
            Err(e) => {
                eprintln!("BPM error: {e}");
                std::process::exit(1);
            }
        };
        Sidecar { bpm, offset_ms: 0 }.save(&sidecar_path);
        (bpm, 0)
    };

    let total_duration = Duration::from_secs(mono.len() as u64 / sample_rate as u64);

    eprintln!("Building waveform...");
    let waveform = WaveformData::compute(mono, sample_rate);

    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();
    let player = Player::connect_new(&mixer);
    let source = TrackingSource::new(stereo, sample_rate, channels);
    player.append(source);

    if let Err(e) = run_tui(filename, bpm, initial_offset_ms, total_duration, waveform, player, &sidecar_path) {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }
}

fn run_tui(
    filename: String,
    bpm: f32,
    initial_offset_ms: i64,
    total_duration: Duration,
    waveform: WaveformData,
    player: Player,
    sidecar_path: &str,
) -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = tui_loop(
        &mut terminal,
        &filename,
        bpm,
        initial_offset_ms,
        total_duration,
        &waveform,
        &player,
        sidecar_path,
    );

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    filename: &str,
    bpm: f32,
    initial_offset_ms: i64,
    total_duration: Duration,
    waveform: &WaveformData,
    player: &Player,
    sidecar_path: &str,
) -> io::Result<()> {
    let beat_period = Duration::from_secs_f64(60.0 / bpm as f64);
    let flash_window = beat_period.mul_f64(0.15);
    let mut offset_ms: i64 = initial_offset_ms;
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;

    loop {
        let pos = player.get_pos();

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
                Paragraph::new(format!("BPM: {bpm:.0}   offset: {:+}ms", offset_ms)),
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
                Paragraph::new("Space: play/pause   +/-: offset   z/Z: zoom   q: quit")
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[5],
            );
        })?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        player.stop();
                        Sidecar { bpm, offset_ms }.save(sidecar_path);
                        break;
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
                    _ => {}
                }
            }
        }

        if player.empty() {
            break;
        }
    }

    Ok(())
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
// Custom rodio Source
// ---------------------------------------------------------------------------

struct TrackingSource {
    samples: std::vec::IntoIter<f32>,
    sample_rate: u32,
    channels: u16,
}

impl TrackingSource {
    fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self { samples: samples.into_iter(), sample_rate, channels }
    }
}

impl Iterator for TrackingSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> { self.samples.next() }
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
// Sidecar
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Sidecar {
    bpm: f32,
    offset_ms: i64,
}

impl Sidecar {
    fn load(path: &str) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn save(&self, path: &str) {
        if let Ok(text) = serde_json::to_string(self) {
            let _ = std::fs::write(path, text);
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
