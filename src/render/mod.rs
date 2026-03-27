use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::audio::WaveformData;
use crate::deck::{
    Deck, NotificationStyle, SpecPalette, TagEditorState,
    TAG_EDITOR_MAX_WIDTH, TAG_EDITOR_MIN_WIDTH, TAG_FIELD_LABELS,
};

pub(crate) const ZOOM_LEVELS: &[f32] = &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
pub(crate) const DEFAULT_ZOOM_IDX: usize = 2; // 4 seconds

/// Interpolate a four-stop spectral palette at `bass` ∈ [0,1] and scale by `brightness`.
/// The interpolated colour is normalised so its dominant channel is always 255 before
/// brightness is applied — this preserves full saturation at all bass ratios and ensures
/// the hue is clearly identifiable even at low brightness.
pub(crate) fn spectral_color(pal: SpecPalette, bass: f32, brightness: f32) -> ratatui::style::Color {
    let stops = [pal.0, pal.1, pal.2, pal.3];
    let t = (bass * 3.0).clamp(0.0, 3.0);
    let seg = (t.floor() as usize).min(stops.len() - 2);
    let f   = t.fract();
    let (a, b) = (stops[seg], stops[seg + 1]);
    let r = a.0 as f32 + f * (b.0 as f32 - a.0 as f32);
    let g = a.1 as f32 + f * (b.1 as f32 - a.1 as f32);
    let b_ = a.2 as f32 + f * (b.2 as f32 - a.2 as f32);
    // Normalise to max channel = 255 so saturation is always 1.0 before dimming.
    let max_ch = r.max(g).max(b_).max(1.0);
    let norm = 255.0 / max_ch * brightness;
    ratatui::style::Color::Rgb(
        (r  * norm).round().clamp(0.0, 255.0) as u8,
        (g  * norm).round().clamp(0.0, 255.0) as u8,
        (b_ * norm).round().clamp(0.0, 255.0) as u8,
    )
}

/// Pre-rendered braille buffer wider than the visible area, enabling smooth scrolling.
/// The UI thread pans a viewport through this stable buffer rather than requesting
/// a full recompute every time the playhead advances by one column.
pub(crate) struct BrailleBuffer {
    pub(crate) grid:            Vec<Vec<u8>>, // rows × buf_cols braille bytes
    pub(crate) bass_ratio:      Vec<f32>,     // per-column bass ratio in [0,1]: 1=bass, 0=treble
    pub(crate) tick:            Vec<u8>,      // per-column tick byte: 0x47=left sub-col, 0xB8=right, 0=none
    pub(crate) cue_buf_col:     Option<usize>,// buffer column of cue point, None if unset or out of range
    pub(crate) buf_cols:        usize,        // total buffer width (= 3 × screen_cols)
    pub(crate) anchor_sample:   usize,        // mono-sample index at the buffer centre
    pub(crate) samples_per_col: usize,        // mono samples represented by each buffer column
}

impl BrailleBuffer {
    pub(crate) fn empty() -> Self {
        Self { grid: Vec::new(), bass_ratio: Vec::new(), tick: Vec::new(), cue_buf_col: None, buf_cols: 0, anchor_sample: 0, samples_per_col: 1 }
    }
}

/// A single background thread that produces two `BrailleBuffer`s — one per
/// deck — each at a `col_samp` scaled by that deck's `bpm / base_bpm` ratio.
/// Scaling by the playback speed means ticks placed at `base_bpm` sample
/// spacing appear at `bpm`-spaced columns, so the tick grids of two decks at
/// the same effective BPM are visually identical.
pub(crate) struct SharedDetailRenderer {
    pub(crate) cols:           Arc<AtomicUsize>,
    pub(crate) rows:           Arc<AtomicUsize>,
    pub(crate) zoom_at:        Arc<AtomicUsize>,
    pub(crate) sample_rate_a:  Arc<AtomicUsize>,
    pub(crate) sample_rate_b:  Arc<AtomicUsize>,
    /// `(bpm / base_bpm) × 65536`, updated on every BPM-changing action.
    pub(crate) speed_ratio_a:  Arc<AtomicUsize>,
    pub(crate) speed_ratio_b:  Arc<AtomicUsize>,
    pub(crate) waveform_a:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    pub(crate) waveform_b:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    pub(crate) display_pos_a:  Arc<AtomicUsize>,
    pub(crate) display_pos_b:  Arc<AtomicUsize>,
    pub(crate) channels_a:     Arc<AtomicUsize>,
    pub(crate) channels_b:     Arc<AtomicUsize>,
    /// Incremented each time a new track is loaded into the slot; signals the
    /// background thread to recompute immediately rather than waiting for drift.
    pub(crate) load_gen_a:     Arc<AtomicUsize>,
    pub(crate) load_gen_b:     Arc<AtomicUsize>,
    /// `base_bpm` as f32 bits; 0 when analysing or unloaded.
    pub(crate) bpm_a:          Arc<AtomicU32>,
    pub(crate) bpm_b:          Arc<AtomicU32>,
    pub(crate) offset_ms_a:    Arc<AtomicI64>,
    pub(crate) offset_ms_b:    Arc<AtomicI64>,
    /// Cue point in mono samples; -1 when unset.
    pub(crate) cue_sample_a:   Arc<AtomicI64>,
    pub(crate) cue_sample_b:   Arc<AtomicI64>,
    /// Gain trim as f32 bits; 1.0 when unset. Peaks in the buffer are pre-scaled
    /// by this value so the detail waveform height tracks gain visually.
    pub(crate) gain_a:         Arc<AtomicU32>,
    pub(crate) gain_b:         Arc<AtomicU32>,
    pub(crate) shared_a:       Arc<Mutex<Arc<BrailleBuffer>>>,
    pub(crate) shared_b:       Arc<Mutex<Arc<BrailleBuffer>>>,
    _stop_guard:    StopOnDrop,
}

struct StopOnDrop(Arc<AtomicBool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
}

impl SharedDetailRenderer {
    pub(crate) fn new(zoom_idx: usize) -> Self {
        let cols           = Arc::new(AtomicUsize::new(0));
        let rows           = Arc::new(AtomicUsize::new(0));
        let zoom_at        = Arc::new(AtomicUsize::new(zoom_idx));
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
        let bpm_a          = Arc::new(AtomicU32::new(0));
        let bpm_b          = Arc::new(AtomicU32::new(0));
        let offset_ms_a    = Arc::new(AtomicI64::new(0));
        let offset_ms_b    = Arc::new(AtomicI64::new(0));
        let cue_sample_a   = Arc::new(AtomicI64::new(-1));
        let cue_sample_b   = Arc::new(AtomicI64::new(-1));
        let gain_a         = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let gain_b         = Arc::new(AtomicU32::new(1.0f32.to_bits()));
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
            let bpm_a_bg     = Arc::clone(&bpm_a);
            let bpm_b_bg     = Arc::clone(&bpm_b);
            let off_ms_a_bg  = Arc::clone(&offset_ms_a);
            let off_ms_b_bg  = Arc::clone(&offset_ms_b);
            let cue_a_bg     = Arc::clone(&cue_sample_a);
            let cue_b_bg     = Arc::clone(&cue_sample_b);
            let gain_a_bg    = Arc::clone(&gain_a);
            let gain_b_bg    = Arc::clone(&gain_b);
            let shared_a_bg  = Arc::clone(&shared_a);
            let shared_b_bg  = Arc::clone(&shared_b);
            let stop_bg      = Arc::clone(&stop);

            thread::spawn(move || {
                let mut last_cols      = 0usize;
                let mut last_rows      = 0usize;
                let mut last_zoom      = usize::MAX;
                let mut last_col_samp_a = 0usize;
                let mut last_col_samp_b = 0usize;
                let mut last_anchor_a  = 0usize;
                let mut last_anchor_b  = 0usize;
                let mut last_gen_a     = usize::MAX;
                let mut last_gen_b     = usize::MAX;
                let mut last_bpm_a: u32  = 0;
                let mut last_bpm_b: u32  = 0;
                let mut last_off_a: i64  = 0;
                let mut last_off_b: i64  = 0;
                let mut last_cue_a: i64  = -1;
                let mut last_cue_b: i64  = -1;
                let mut last_gain_a: u32 = 1.0f32.to_bits();
                let mut last_gain_b: u32 = 1.0f32.to_bits();

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

                    let gen_a    = gen_a_bg.load(Ordering::Relaxed);
                    let gen_b    = gen_b_bg.load(Ordering::Relaxed);
                    let bpm_a_raw = bpm_a_bg.load(Ordering::Relaxed);
                    let bpm_b_raw = bpm_b_bg.load(Ordering::Relaxed);
                    let off_ms_a  = off_ms_a_bg.load(Ordering::Relaxed);
                    let off_ms_b  = off_ms_b_bg.load(Ordering::Relaxed);
                    let cue_raw_a  = cue_a_bg.load(Ordering::Relaxed);
                    let cue_raw_b  = cue_b_bg.load(Ordering::Relaxed);
                    let gain_raw_a = gain_a_bg.load(Ordering::Relaxed);
                    let gain_raw_b = gain_b_bg.load(Ordering::Relaxed);
                    let must_recompute = cols != last_cols
                        || rows != last_rows
                        || zoom != last_zoom
                        || col_samp_a != last_col_samp_a
                        || col_samp_b != last_col_samp_b
                        || drift_a >= cols * 3 / 4
                        || drift_b >= cols * 3 / 4
                        || gen_a != last_gen_a
                        || gen_b != last_gen_b
                        || bpm_a_raw != last_bpm_a
                        || bpm_b_raw != last_bpm_b
                        || off_ms_a != last_off_a
                        || off_ms_b != last_off_b
                        || cue_raw_a != last_cue_a
                        || cue_raw_b != last_cue_b
                        || gain_raw_a != last_gain_a
                        || gain_raw_b != last_gain_b;

                    if must_recompute {
                        let buf_cols = cols * 5;

                        let wf_a: Option<Arc<WaveformData>> = wf_a_bg.lock().unwrap().clone();
                        let wf_b: Option<Arc<WaveformData>> = wf_b_bg.lock().unwrap().clone();

                        let anchor_a = (pos_a / col_samp_a) * col_samp_a;
                        let anchor_b = (pos_b / col_samp_b) * col_samp_b;

                        let tick_view_start_a = anchor_a as f64 - (buf_cols / 2) as f64 * col_samp_a as f64;
                        let tick_view_start_b = anchor_b as f64 - (buf_cols / 2) as f64 * col_samp_b as f64;
                        let compute_cue_buf_col = |cue_raw: i64, anchor: usize, col_samp: usize| -> Option<usize> {
                            if cue_raw < 0 || col_samp == 0 { return None; }
                            let delta = cue_raw - anchor as i64;
                            let col = buf_cols as i64 / 2 + delta.div_euclid(col_samp as i64);
                            if col >= 0 && (col as usize) < buf_cols { Some(col as usize) } else { None }
                        };
                        let gain_a = f32::from_bits(gain_raw_a);
                        let gain_b = f32::from_bits(gain_raw_b);
                        let scale_peaks = |peaks: Vec<(f32, f32)>, g: f32| -> Vec<(f32, f32)> {
                            peaks.into_iter().map(|(mn, mx)| (mn * g, mx * g)).collect()
                        };
                        let buf_a = Arc::new(BrailleBuffer {
                            grid: render_braille(
                                &scale_peaks(peaks_for_slot(&wf_a, anchor_a, col_samp_a, buf_cols), gain_a),
                                rows, buf_cols,
                            ),
                            bass_ratio:      spectral_for_slot(&wf_a, anchor_a, col_samp_a, buf_cols, sr_a as u32),
                            tick:            compute_tick_display(buf_cols, col_samp_a, tick_view_start_a,
                                                 bpm_a_raw == 0, f32::from_bits(bpm_a_raw), sr_a as u32, off_ms_a),
                            cue_buf_col:     compute_cue_buf_col(cue_raw_a, anchor_a, col_samp_a),
                            buf_cols,
                            anchor_sample:   anchor_a,
                            samples_per_col: col_samp_a,
                        });
                        let buf_b = Arc::new(BrailleBuffer {
                            grid: render_braille(
                                &scale_peaks(peaks_for_slot(&wf_b, anchor_b, col_samp_b, buf_cols), gain_b),
                                rows, buf_cols,
                            ),
                            bass_ratio:      spectral_for_slot(&wf_b, anchor_b, col_samp_b, buf_cols, sr_b as u32),
                            tick:            compute_tick_display(buf_cols, col_samp_b, tick_view_start_b,
                                                 bpm_b_raw == 0, f32::from_bits(bpm_b_raw), sr_b as u32, off_ms_b),
                            cue_buf_col:     compute_cue_buf_col(cue_raw_b, anchor_b, col_samp_b),
                            buf_cols,
                            anchor_sample:   anchor_b,
                            samples_per_col: col_samp_b,
                        });

                        *shared_a_bg.lock().unwrap() = buf_a;
                        *shared_b_bg.lock().unwrap() = buf_b;

                        last_cols       = cols;
                        last_rows       = rows;
                        last_zoom       = zoom;
                        last_col_samp_a = col_samp_a;
                        last_col_samp_b = col_samp_b;
                        last_anchor_a   = anchor_a;
                        last_anchor_b   = anchor_b;
                        last_gen_a      = gen_a;
                        last_gen_b      = gen_b;
                        last_bpm_a      = bpm_a_raw;
                        last_bpm_b      = bpm_b_raw;
                        last_off_a      = off_ms_a;
                        last_off_b      = off_ms_b;
                        last_cue_a      = cue_raw_a;
                        last_cue_b      = cue_raw_b;
                        last_gain_a     = gain_raw_a;
                        last_gain_b     = gain_raw_b;
                    }

                    thread::sleep(Duration::from_millis(8));
                }
            });
        }

        SharedDetailRenderer {
            cols, rows, zoom_at,
            sample_rate_a, sample_rate_b,
            speed_ratio_a, speed_ratio_b,
            waveform_a, waveform_b,
            display_pos_a, display_pos_b,
            channels_a, channels_b,
            load_gen_a, load_gen_b,
            bpm_a, bpm_b,
            offset_ms_a, offset_ms_b,
            cue_sample_a, cue_sample_b,
            gain_a, gain_b,
            shared_a, shared_b,
            _stop_guard: stop_guard,
        }
    }

    pub(crate) fn set_deck(&self, slot: usize, wf: Arc<WaveformData>, channels: u16, sample_rate: u32) {
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

    pub(crate) fn store_speed_ratio(&self, slot: usize, bpm: f32, base_bpm: f32) {
        let ratio = ((bpm / base_bpm) as f64 * 65536.0) as usize;
        match slot {
            0 => self.speed_ratio_a.store(ratio, Ordering::Relaxed),
            _ => self.speed_ratio_b.store(ratio, Ordering::Relaxed),
        }
    }

    pub(crate) fn store_cue(&self, slot: usize, cue_sample: Option<usize>) {
        let raw = cue_sample.map_or(-1, |s| s as i64);
        match slot {
            0 => self.cue_sample_a.store(raw, Ordering::Relaxed),
            _ => self.cue_sample_b.store(raw, Ordering::Relaxed),
        }
    }

    pub(crate) fn store_tempo(&self, slot: usize, base_bpm: f32, offset_ms: i64, analysing: bool) {
        let bpm_raw = if analysing { 0.0f32 } else { base_bpm }.to_bits();
        match slot {
            0 => {
                self.bpm_a.store(bpm_raw, Ordering::Relaxed);
                self.offset_ms_a.store(offset_ms, Ordering::Relaxed);
            }
            _ => {
                self.bpm_b.store(bpm_raw, Ordering::Relaxed);
                self.offset_ms_b.store(offset_ms, Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn store_gain(&self, slot: usize, gain_linear: f32) {
        match slot {
            0 => self.gain_a.store(gain_linear.to_bits(), Ordering::Relaxed),
            _ => self.gain_b.store(gain_linear.to_bits(), Ordering::Relaxed),
        }
    }
}

fn cache_indicator_spans(deck: &Deck, vinyl_mode: bool) -> Vec<Span<'static>> {
    let lit  = Style::default().fg(spectral_color(deck.display.palette, 0.0, 0.45));
    let dim  = Style::default().fg(spectral_color(deck.display.palette, 0.0, 0.18));
    let dark = Style::default().fg(Color::Rgb(50, 50, 50));
    let indicator_style = |active: bool| match (active, vinyl_mode) {
        (true,  false) => lit,
        (true,  true)  => dim,
        (false, _)     => dark,
    };
    vec![
        Span::styled("[BPM]",  indicator_style(deck.tempo.bpm_established)),
        Span::styled("[Tick]", indicator_style(deck.tempo.offset_established || deck.cue_sample.is_some())),
        Span::styled("[Cue]",  indicator_style(deck.cue_sample.is_some())),
    ]
}

pub(crate) fn notification_line_for_deck(deck: &Deck, content_width: usize, vinyl_mode: bool) -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
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
            NotificationStyle::Success => Color::Green,
        };
        Line::from(Span::styled(n.message.clone(), Style::default().fg(color)))
    } else if deck.rename_offer_active() {
        let elapsed = deck.rename_offer_started.unwrap().elapsed().as_secs();
        let (offer, offer_style) = if elapsed < 10 {
            let secs_left = 10 - elapsed;
            (format!("rename? [y]  ({}s)", secs_left), Style::default().fg(Color::Red))
        } else {
            ("rename? [y]".to_string(), dim)
        };
        let track_name = deck.track_name.clone();
        let indicators = cache_indicator_spans(deck, vinyl_mode);
        let indicators_w = 16; // "[BPM][Tick][Cue]"
        let left_w  = track_name.chars().count();
        let right_w = offer.chars().count() + 1 + indicators_w; // space + indicators
        let spacer_w = content_width.saturating_sub(left_w + right_w).max(1);
        let mut spans = vec![
            Span::styled(track_name, Style::default().fg(spectral_color(deck.display.palette, 0.0, 0.85))),
            Span::raw(" ".repeat(spacer_w)),
            Span::styled(offer, offer_style),
            Span::raw(" "),
        ];
        spans.extend(indicators);
        Line::from(spans)
    } else {
        let track_name = deck.track_name.clone();
        let indicators = cache_indicator_spans(deck, vinyl_mode);
        let indicators_w = 16; // "[BPM][Tick][Cue]"
        let left_w   = track_name.chars().count();
        let spacer_w = content_width.saturating_sub(left_w + indicators_w).max(1);
        let mut spans = vec![
            Span::styled(track_name, Style::default().fg(spectral_color(deck.display.palette, 0.0, 0.85))),
            Span::raw(" ".repeat(spacer_w)),
        ];
        spans.extend(indicators);
        Line::from(spans)
    }
}

pub(crate) fn notification_line_empty() -> Line<'static> {
    Line::from(Span::styled(
        "no track — press z to open the file browser",
        Style::default().fg(Color::Rgb(60, 60, 60)),
    ))
}

pub(crate) fn info_line_for_deck(
    deck: &Deck,
    frame_count: usize,
    beat_on: bool,
    analysing: bool,
    _label_style: Style,
    bar_width: u16,
    vinyl_mode: bool,
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
    let tap_flash_on = deck.tap.last_tap_wall
        .map_or(false, |t| t.elapsed().as_millis() < 150);
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
    // Beat flash is suppressed in vinyl mode.
    let beat_active = !vinyl_mode && beat_on;
    let beat_style = if beat_active {
        Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0))
    } else {
        dim
    };
    // Percentage display: vinyl mode always; beat mode when no BPM established.
    let show_percentage = vinyl_mode || !deck.tempo.bpm_established;

    // --- Left group ---
    let left_spans: Vec<Span<'static>> = {
        let mut spans = vec![Span::styled(format!("{play_icon}  "), dim)];
        if show_percentage {
            let pct = if vinyl_mode {
                (deck.tempo.vinyl_speed - 1.0) * 100.0
            } else {
                (deck.tempo.bpm / deck.tempo.base_bpm - 1.0) * 100.0
            };
            let rounded = (pct * 10.0).round() / 10.0;
            let pct_str = if rounded == 0.0 {
                "0.0%".to_string()
            } else if rounded > 0.0 {
                format!("+{:.1}%", rounded)
            } else {
                format!("{:.1}%", rounded)
            };
            spans.push(Span::styled(pct_str, beat_style));
            // In beat mode without BPM, offset and metronome remain visible (dormant).
            // In vinyl mode they are hidden entirely.
            if !vinyl_mode {
                if deck.metronome_mode {
                    spans.push(Span::styled("\u{266A}", Style::default().fg(Color::Red)));
                }
                spans.push(Span::styled(format!("  {:+}ms", deck.tempo.offset_ms), dim));
            }
        } else {
            // base_bpm adjusts in 0.01 steps → 2dp; playback bpm adjusts in 0.1 steps → 1dp.
            let adjusted = (deck.tempo.bpm - deck.tempo.base_bpm).abs() >= 0.05;
            if adjusted {
                spans.push(Span::styled(format!("{:.2} ", deck.tempo.base_bpm), dim));
                spans.push(Span::styled("(", dim));
                spans.push(Span::styled(format!("{:.1}", deck.tempo.bpm), beat_style));
                spans.push(Span::styled(")", dim));
            } else {
                spans.push(Span::styled(format!("{:.2}", deck.tempo.base_bpm), beat_style));
            }
            if deck.metronome_mode {
                spans.push(Span::styled("\u{266A}", Style::default().fg(Color::Red)));
            }
            spans.push(Span::styled(format!("  {:+}ms", deck.tempo.offset_ms), dim));
        }
        if !tap_str.is_empty() {
            if tap_flash_on {
                let tap_flash_style = Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0));
                spans.push(Span::styled(" ", dim));
                spans.push(Span::styled(format!(" tap:{} ", deck.tap.tap_times.len()), tap_flash_style));
            } else {
                spans.push(Span::styled(tap_str.clone(), dim));
            }
        }
        spans
    };

    // --- Right group ---
    let mut right_spans: Vec<Span<'static>> = Vec::new();
    if !nudge_str.is_empty() {
        right_spans.push(Span::styled(nudge_str.to_string(), dim));
    }
    const LEVEL_BARS: [char; 8] = ['▁','▂','▃','▄','▅','▆','▇','█'];
    let level_idx = ((deck.volume * 7.0).round() as usize).min(7);
    let level_char = LEVEL_BARS[level_idx];
    let t = level_idx as f32 / 7.0;
    let level_style = Style::default()
        .fg(Color::Rgb((60.0 + 195.0 * t).round() as u8, (50.0 + 165.0 * t).round() as u8, 0))
        .bg(Color::Rgb((40.0 * t).round() as u8, (33.0 * t).round() as u8, 0));
    let bracket_style = Style::default().fg(Color::Rgb(140, 140, 140));
    if deck.pfl_level > 0 {
        right_spans.push(Span::styled("  PFL", Style::default().fg(Color::Cyan)));
    }
    right_spans.push(Span::styled("  level:", dim));
    right_spans.push(Span::styled("\u{2595}", bracket_style));
    right_spans.push(Span::styled(level_char.to_string(), level_style));
    right_spans.push(Span::styled("\u{258F}", bracket_style));
    {
        const GAIN_CHARS: [char; 7] = ['▁','▂','▃','▄','▅','▆','▇'];
        let idx = ((deck.gain_db as i32 + 12) * 6 / 24).clamp(0, 6) as usize;
        let gain_style = if deck.gain_db == 0 {
            Style::default().fg(Color::Rgb(45, 45, 45))
        } else {
            Style::default().fg(Color::Rgb(180, 140, 0))
        };
        right_spans.push(Span::styled(GAIN_CHARS[idx].to_string(), gain_style));
    }
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
        // dB/oct indicator: fixed 2-char field — visible when filter active, blank otherwise.
        let slope_str = if deck.filter_offset != 0 { match deck.filter_poles { 4 => "24", _ => "12" } } else { "  " };
        right_spans.push(Span::styled(slope_str, dim));
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

pub(crate) fn info_line_empty(bar_width: u16) -> Line<'static> {
    let dim = Style::default().fg(Color::Rgb(60, 60, 60));
    let left  = Span::styled("⏸  ---  +0ms", dim);
    let right = Span::styled("zoom:---", dim);
    let lw = left.content.chars().count();
    let rw = right.content.chars().count();
    let spacer = " ".repeat((bar_width as usize).saturating_sub(lw + rw).max(1));
    Line::from(vec![left, Span::raw(spacer), right])
}

pub(crate) fn overview_for_deck(
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
    let playhead_frac = if deck.total_duration == 0.0 {
        0.0
    } else {
        (display_samp / deck.audio.sample_rate as f64 / deck.total_duration).clamp(0.0, 1.0)
    };
    let playhead_col = ((playhead_frac * overview_width as f64).round() as usize)
        .min(overview_width.saturating_sub(1));
    let cue_col: Option<usize> = deck.cue_sample.map(|samp| {
        let frac = (samp as f64 / deck.audio.sample_rate as f64
            / deck.total_duration).clamp(0.0, 1.0);
        ((frac * overview_width as f64).round() as usize)
            .min(overview_width.saturating_sub(1))
    });

    let gain_linear = 10f32.powf(deck.gain_db as f32 / 20.0);
    let hires: Vec<((f32, f32), f32)> = (0..overview_width * 2)
        .map(|col| {
            let idx = (col * total_peaks / (overview_width * 2).max(1)).min(total_peaks.saturating_sub(1));
            let (min_v, max_v) = deck.audio.waveform.peaks[idx];
            let bass = deck.audio.waveform.bass_ratio[idx];
            ((min_v * gain_linear, max_v * gain_linear), bass)
        })
        .collect();
    let ov_peaks_hires: Vec<(f32, f32)> = hires.iter().map(|(p, _)| *p).collect();
    let ov_bass_hires: Vec<f32>          = hires.iter().map(|(_, b)| *b).collect();
    let hires_buf = render_braille(&ov_peaks_hires, overview_height, overview_width * 2);
    let ov_braille: Vec<Vec<u8>> = hires_buf.iter()
        .map(|row| (0..overview_width).map(|c| (row[c * 2] & 0x47) | (row[c * 2 + 1] & 0xB8)).collect())
        .collect();
    let ov_bass: Vec<f32> = (0..overview_width)
        .map(|c| (ov_bass_hires[c * 2] + ov_bass_hires[c * 2 + 1]) / 2.0)
        .collect();
    let (bar_cols, bar_times, bars_per_tick): (Vec<usize>, Vec<f64>, u32) = if !analysing {
        bar_tick_cols(deck.tempo.base_bpm as f64, deck.tempo.offset_ms, deck.total_duration, overview_width)
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
                        (Color::Rgb(255, 0, 255), '\u{28FF}')
                    } else {
                        (Color::Rgb(255, 255, 255), '\u{28FF}')
                    }
                } else if c == playhead_col {
                    (Color::Rgb(255, 255, 255), '\u{28FF}')
                } else if cue_col == Some(c) {
                    (Color::Rgb(255, 0, 255), '\u{28FF}')
                } else if bar_cols.contains(&c) {
                    if warn_beat_on {
                        (Color::Rgb(120, 60, 60), '│')
                    } else if warning_active {
                        (Color::Rgb(40, 20, 20), '│')
                    } else {
                        (Color::DarkGray, '│')
                    }
                } else {
                    let spectral = spectral_color(deck.display.palette, ov_bass[c], 0.8);
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

fn empty_deck_mesh_line(w: usize, bg: Color, fg: Color) -> Line<'static> {
    // U+2895 has dots 1,3,5,8 — a checkerboard within the 2×4 cell (•./. •/•./. •).
    // Tiling identical characters continues the alternating pattern seamlessly in
    // both directions, so no per-column logic is needed.
    let s: String = std::iter::repeat('\u{2895}').take(w).collect();
    Line::from(Span::styled(s, Style::default().fg(fg).bg(bg)))
}

pub(crate) fn overview_empty(rect: ratatui::layout::Rect, deck_slot: usize) -> Vec<Line<'static>> {
    let w = rect.width as usize;
    let h = rect.height as usize;
    let bg = Color::Rgb(11, 11, 15);
    let fg = if deck_slot == 0 {
        Color::Rgb(26, 26, 36)
    } else {
        Color::Rgb(17, 17, 24)
    };
    vec![empty_deck_mesh_line(w, bg, fg); h]
}

pub(crate) fn render_detail_empty(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    deck_slot: usize,
) {
    let w = area.width as usize;
    let h = area.height as usize;
    let bg = Color::Rgb(11, 11, 15);
    let fg = if deck_slot == 0 {
        Color::Rgb(26, 26, 36)
    } else {
        Color::Rgb(17, 17, 24)
    };
    let lines: Vec<Line<'static>> = vec![empty_deck_mesh_line(w, bg, fg); h];
    frame.render_widget(Paragraph::new(lines), area);
}

pub(crate) fn popup_area(width: u16, height: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect { x, y, width: width.min(area.width), height: height.min(area.height) }
}

pub(crate) fn render_editor_field(label: &'static str, text: &str, active: bool, cursor: usize, text_width: usize) -> Vec<Line<'static>> {
    let label_style  = Style::default().fg(Color::Rgb(40, 60, 100));
    let text_style   = Style::default().fg(if active { Color::White } else { Color::Rgb(60, 80, 120) });
    let cursor_style = Style::default().fg(Color::Black).bg(Color::Yellow);
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());

    // Split into visual rows; always at least one so an empty field still renders.
    let rows: Vec<&[char]> = if chars.is_empty() {
        vec![&[]]
    } else {
        chars.chunks(text_width).collect()
    };

    rows.iter().enumerate().map(|(row_idx, row_chars)| {
        let start      = row_idx * text_width;
        let row_len    = row_chars.len();
        let is_last    = row_idx == rows.len() - 1;
        let prefix     = if row_idx == 0 { format!("{label}: ") } else { " ".repeat(9) };
        let cursor_here = active && (
            (cursor >= start && cursor < start + row_len) ||
            (is_last && cursor == chars.len())
        );
        if cursor_here {
            let local = cursor - start;
            let before: String = row_chars[..local].iter().collect();
            let (at_cur, after): (String, String) = if local < row_len {
                (row_chars[local].to_string(), row_chars[local + 1..].iter().collect())
            } else {
                (" ".to_string(), String::new())
            };
            Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(before, text_style),
                Span::styled(at_cur, cursor_style),
                Span::styled(after, text_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(row_chars.iter().collect::<String>(), text_style),
            ])
        }
    }).collect()
}

pub(crate) fn section_divider(label: &'static str, inner_width: usize) -> Line<'static> {
    let fill = "─".repeat(inner_width.saturating_sub(4 + label.len()));
    Line::from(vec![
        Span::styled("── ", Style::default().fg(Color::Rgb(40, 60, 100))),
        Span::styled(label, Style::default().fg(Color::Rgb(80, 110, 160))),
        Span::styled(format!(" {fill}"), Style::default().fg(Color::Rgb(40, 60, 100))),
    ])
}

pub(crate) fn render_tag_editor(frame: &mut ratatui::Frame, editor: &TagEditorState, full_area: ratatui::layout::Rect) {
    let popup_width = full_area.width.clamp(TAG_EDITOR_MIN_WIDTH, TAG_EDITOR_MAX_WIDTH);
    let text_width  = popup_width as usize - 2 - 9; // inner − label prefix
    let inner_width = popup_width as usize - 2;
    let label_dim = Style::default().fg(Color::Rgb(40, 60, 100));
    let hint_dim  = Style::default().fg(Color::Rgb(40, 60, 100));
    let proposed  = editor.preview();
    let with_ext  = |stem: &str| -> String {
        if editor.extension.is_empty() { stem.to_string() }
        else { format!("{stem}.{}", editor.extension) }
    };
    let mut lines: Vec<Line<'static>> = std::iter::once(section_divider("Tags", inner_width))
        .chain(TAG_FIELD_LABELS.iter().enumerate()
            .flat_map(|(i, &label)| {
                let (val, cur) = &editor.fields[i];
                render_editor_field(label, val, editor.active_field == i, *cur, text_width)
            }))
        .collect();
    lines.push(section_divider("Filename", inner_width));
    lines.push(Line::from(vec![
        Span::styled(" Current: ", label_dim),
        Span::styled(with_ext(&editor.current_stem), Style::default().fg(Color::Rgb(60, 80, 120))),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Proposed: ", label_dim),
        Span::styled(with_ext(&proposed), Style::default().fg(Color::Yellow)),
    ]));
    if let Some(ref err) = editor.collision_error {
        lines.push(Line::from(Span::styled(format!(" \u{26a0} {err}"), Style::default().fg(Color::Red))));
    } else {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled("          Enter to confirm  Esc to cancel", hint_dim)));
    let popup_height = (lines.len() as u16 + 2).min(full_area.height); // +2 for borders
    let popup = popup_area(popup_width, popup_height, full_area);
    let navy = Style::default().bg(Color::Rgb(20, 20, 38));
    let blue = Color::Rgb(40, 60, 100);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Edit tags and rename file ", Style::default().fg(Color::Yellow)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(blue))
        .style(navy);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(Paragraph::new(lines), inner);
}

// Compose the braille display row for the shared tick strip.
// Each tick occupies two adjacent characters: a main char at column c and a spillover into c+1.
// Deck A (up): 1 tip dot (row 1, sub-col position) + 3-wide base (row 2).
// Deck B (down): 3-wide base (row 3) + 1 tip dot (row 4, sub-col position).
// Left/right sub-column is determined by bit 0 of the raw tick byte (0x47=left, 0xB8=right).
pub(crate) fn compose_shared_tick_row(tick_a: &[u8], tick_b: &[u8], width: usize) -> Vec<u8> {
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

/// Extract a screen-width slice of tick data from a pre-rendered buffer, applying the
/// same half-column viewport transform as the waveform so ticks stay locked to peaks.
/// When `sub_col` is true (the viewport is offset by one half-column), tick bytes are
/// shifted: right sub-col (0xB8) at buffer column c becomes left sub-col (0x47) at
/// screen column c, and left sub-col (0x47) at buffer column c becomes right sub-col
/// (0xB8) at screen column c−1.
pub(crate) fn extract_tick_viewport(
    buf:        &BrailleBuffer,
    display_pos: usize,
    centre_col: usize,
    width:      usize,
) -> Vec<u8> {
    if buf.samples_per_col == 0 || buf.tick.is_empty() {
        return vec![0u8; width];
    }
    let half_col      = buf.samples_per_col as f64 / 2.0;
    let delta         = display_pos as i64 - buf.anchor_sample as i64;
    let delta_half    = (delta as f64 / half_col).round() as i64;
    let delta_cols    = delta_half.div_euclid(2);
    let sub_col       = delta_half.rem_euclid(2) != 0;
    let viewport_off  = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
    let need          = if sub_col { width + 1 } else { width };
    if viewport_off < 0 || (viewport_off as usize) + need > buf.buf_cols {
        return vec![0u8; width];
    }
    let start = viewport_off as usize;
    if !sub_col {
        buf.tick[start..start + width].to_vec()
    } else {
        // With a half-column shift: 0xB8 (right) at buf col → 0x47 (left) at same screen col;
        // 0x47 (left) at buf col → 0xB8 (right) at previous screen col.
        let mut out = vec![0u8; width];
        for c in 0..=width {
            let b = buf.tick[start + c];
            if b == 0xB8 && c < width { out[c]     = 0x47; }
            if b == 0x47 && c > 0     { out[c - 1] = 0xB8; }
        }
        out
    }
}

pub(crate) fn compute_tick_display(
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

pub(crate) fn peaks_for_slot(
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

/// Compute per-column bass ratio directly from raw samples using an IIR low-pass,
/// smoothed with a box filter to avoid sharp colour transitions at wide zoom.
pub(crate) fn spectral_for_slot(
    wf: &Option<Arc<WaveformData>>,
    anchor: usize,
    col_samp: usize,
    buf_cols: usize,
    sample_rate: u32,
) -> Vec<f32> {
    let Some(wf) = wf else {
        return vec![0.5; buf_cols];
    };
    let mono  = &wf.mono;
    let alpha = {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * 250.0);
        let dt = 1.0 / sample_rate as f32;
        dt / (rc + dt)
    };
    let bass_raw: Vec<f32> = (0..buf_cols).map(|c| {
        let offset    = c as i64 - (buf_cols / 2) as i64;
        let raw_start = anchor as i64 + offset * col_samp as i64;
        if raw_start < 0 || raw_start as usize >= mono.len() {
            return 0.5;
        }
        let samp_start = raw_start as usize;
        let chunk = &mono[samp_start..(samp_start + col_samp).min(mono.len())];
        if chunk.is_empty() { return 0.5; }
        let total_energy: f32 = chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32;
        let mut lp = 0.0f32;
        let lp_energy: f32 = chunk.iter().map(|&s| { lp += alpha * (s - lp); lp * lp })
            .sum::<f32>() / chunk.len() as f32;
        (lp_energy / (total_energy + 1e-10)).clamp(0.0, 1.0)
    }).collect();
    box_smooth(&bass_raw, 3)
}


pub(crate) fn box_smooth(v: &[f32], radius: usize) -> Vec<f32> {
    let n = v.len();
    (0..n).map(|i| {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius + 1).min(n);
        v[lo..hi].iter().sum::<f32>() / (hi - lo) as f32
    }).collect()
}

/// Takes the right dot-column of `a` (bits 3,4,5,7) as the new left column (bits 0,1,2,6)
/// and the left dot-column of `b` (bits 0,1,2,6) as the new right column (bits 3,4,5,7).
pub(crate) fn shift_braille_half(a: u8, b: u8) -> u8 {
    let left  = ((a >> 3) & 0x07) | ((a >> 1) & 0x40);
    let right = ((b & 0x07) << 3) | ((b & 0x40) << 1);
    left | right
}

pub(crate) fn render_braille(peaks: &[(f32, f32)], rows: usize, cols: usize) -> Vec<Vec<u8>> {
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

    for (c, &(min_val, max_val)) in peaks.iter().take(cols).enumerate() {
        let clamped_max = max_val.min(1.0);
        let clamped_min = min_val.max(-1.0);
        if clamped_min > clamped_max { continue; }
        // Map y ∈ [-1, 1] → dot row ∈ [0, total_dots); y=1 is top (row 0).
        let top_dot = ((1.0 - clamped_max) / 2.0 * total_dots as f32) as usize;
        let bot_dot = {
            let raw = (((1.0 - clamped_min) / 2.0 * total_dots as f32) as usize)
                .min(total_dots - 1);
            if raw > top_dot && raw + top_dot >= total_dots { raw - 1 } else { raw }
        };
        for d in top_dot..=bot_dot { set_dot(c, d); }
    }
    grid
}

/// Return the column indices of bar-tick lines within the overview, and the bars-per-tick interval.
///
/// Starts at 4 bars and doubles until all adjacent ticks are at least 2 columns apart
/// (leaving at least 1 blank character gap between every pair of markers).
pub(crate) fn bar_tick_cols(bpm: f64, offset_ms: i64, total_secs: f64, cols: usize) -> (Vec<usize>, Vec<f64>, u32) {
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

pub(crate) fn render_detail_waveform(
    frame: &mut ratatui::Frame,
    buf: &Arc<BrailleBuffer>,
    deck: &mut Deck,
    detail_area: ratatui::layout::Rect,
    display_cfg: &crate::config::DisplayConfig,
    display_pos_samp: usize,
    palette: SpecPalette,
) {
    let detail_width      = detail_area.width  as usize;
    let detail_panel_rows = detail_area.height as usize;
    let buf = Arc::clone(buf);
    let centre_col = ((detail_width as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
        .clamp(0, detail_width.saturating_sub(1));

    let half_col_samp: f64 = buf.samples_per_col as f64 / 2.0;
    let mut sub_col = false;
    let viewport_start: Option<usize> = if buf.buf_cols >= detail_width && buf.samples_per_col > 0 {
        let delta = display_pos_samp as i64 - buf.anchor_sample as i64;
        let delta_half = (delta as f64 / half_col_samp).round() as i64;
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

    // Cue column is pre-computed by the background thread in buffer space, using the
    // same anchor and samples_per_col as the waveform and ticks. Map to screen here
    // via viewport_start — identical to how ticks are handled.
    let cue_screen_col: Option<usize> = viewport_start.and_then(|vs| {
        buf.cue_buf_col.and_then(|cbc| {
            if cbc >= vs && cbc < vs + detail_width { Some(cbc - vs) } else { None }
        })
    });

    let waveform_rows = detail_panel_rows;

    let detail_lines: Vec<Line<'static>> = (0..waveform_rows)
        .map(|r| {
            // buf_r maps directly: row 0 → buffer row 0.
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
            let actual_rows = buf.grid.len().min(waveform_rows);
            let is_edge_row = r == 0 || r + 1 == actual_rows;
            let row = match row_slice {
                None => return Line::from(Span::raw("\u{2800}".repeat(detail_width))),
                Some(s) => s,
            };
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_color = Color::Reset;
            for (c, &byte) in row.iter().enumerate() {
                let buf_col  = viewport_start.unwrap_or(0) + c;
                let bass     = buf.bass_ratio.get(buf_col).copied().unwrap_or(0.5);
                let spectral = spectral_color(palette, bass, 1.0);
                let (color, ch) = if c == centre_col && cue_screen_col == Some(c) {
                    if is_edge_row {
                        (Color::Rgb(255, 0, 255), '\u{28FF}')
                    } else {
                        (Color::Rgb(255, 255, 255), '\u{28FF}')
                    }
                } else if c == centre_col {
                    (Color::Rgb(255, 255, 255), '\u{28FF}')
                } else if cue_screen_col == Some(c) {
                    (Color::Rgb(255, 0, 255), '\u{28FF}')
                } else {
                    (spectral, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
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

    frame.render_widget(Paragraph::new(detail_lines), detail_area);
}

pub(crate) fn render_shared_tick_row(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    tick_a: &[u8],
    tick_b: &[u8],
) {
    let w = area.width as usize;
    let display_row = compose_shared_tick_row(tick_a, tick_b, w);
    let s: String = display_row.iter().map(|&byte| {
        if byte != 0 { char::from_u32(0x2800 | byte as u32).unwrap_or(' ') } else { ' ' }
    }).collect();
    frame.render_widget(Paragraph::new(Line::from(Span::styled(s, Style::default().fg(Color::Gray)))), area);
}
