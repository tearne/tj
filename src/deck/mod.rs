use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::Arc;
use std::time::Instant;

use ratatui::layout::Rect;
use rodio::Player;

use crate::audio::{butterworth_biquad, SeekHandle, WaveformData, FILTER_CUTOFFS_HZ};

pub(crate) type Rgb = (u8, u8, u8);
/// Four-stop spectral palette: (treble, mid-treble, mid-bass, bass).
/// Stops are ordered from treble (bass_ratio=0) to bass (bass_ratio=1).
pub(crate) type SpecPalette = (Rgb, Rgb, Rgb, Rgb);

/// Colour schemes: (name, deck_a_palette, deck_b_palette).
/// The first scheme is the default; cycling rotates through all.
pub(crate) const PALETTE_SCHEMES: &[(&str, SpecPalette, SpecPalette)] = &[
    ("amber/cyan",
     ((  0, 255, 255), (  0, 255, 120), (220, 255,   0), (255, 120,   0)),  // cyan → teal → gold → amber
     ((  0, 255, 255), (  0, 255, 120), (220, 255,   0), (255, 120,   0))), // B: same
    ("warm-rainbow",
     (( 60, 255,   0), (220, 255,   0), (255, 140,   0), (255,  20,   0)),  // lime → yellow → orange → red
     (( 60, 255,   0), (220, 255,   0), (255, 140,   0), (255,  20,   0))), // B: same
    ("sunset",
     ((220, 220,   0), (255, 140,   0), (255,  20,  80), (180,   0, 200)),  // gold → orange → crimson → violet
     ((220, 220,   0), (255, 140,   0), (255,  20,  80), (180,   0, 200))), // B: same
    ("ember",
     ((220, 200,   0), (255, 120,   0), (200,  20,   0), ( 80,   0,   0)),  // gold → orange → red → deep-red
     ((220, 200,   0), (255, 120,   0), (200,  20,   0), ( 80,   0,   0))), // B: same
];


pub(crate) struct DeckAudio {
    pub(crate) player: Player,
    pub(crate) seek_handle: SeekHandle,
    pub(crate) mono: Arc<Vec<f32>>,
    pub(crate) waveform: Arc<WaveformData>,
    pub(crate) sample_rate: u32,
    pub(crate) filter_offset_shared: Arc<AtomicI32>,
    pub(crate) filter_state_reset: Arc<AtomicBool>,
}

pub(crate) struct TempoState {
    pub(crate) bpm: f32,
    pub(crate) base_bpm: f32,
    pub(crate) offset_ms: i64,
    pub(crate) bpm_rx: std::sync::mpsc::Receiver<(String, f32, i64, bool)>,
    pub(crate) analysis_hash: Option<String>,
    pub(crate) bpm_established: bool,
    pub(crate) pending_bpm: Option<(String, f32, i64, Instant)>,
    pub(crate) redetecting: bool,
    pub(crate) redetect_saved_hash: Option<String>,
    pub(crate) background_rx: Option<std::sync::mpsc::Receiver<(String, f32, i64, bool)>>,
    /// Absolute playback speed multiplier (1.0 = nominal). Used in vinyl mode and when
    /// no BPM is established. Independent of BPM state; passed directly to `player.set_speed`.
    pub(crate) vinyl_speed: f32,
}

pub(crate) struct TapState {
    pub(crate) tap_times: Vec<f64>,
    pub(crate) last_tap_wall: Option<Instant>,
    pub(crate) was_tap_active: bool,
}

pub(crate) struct DisplayState {
    pub(crate) smooth_display_samp: f64,
    pub(crate) last_scrub_samp: f64,
    pub(crate) last_viewport_start: usize,
    pub(crate) overview_rect: Rect,
    pub(crate) last_bar_cols: Vec<usize>,
    pub(crate) last_bar_times: Vec<f64>,
    pub(crate) palette: SpecPalette,
}

pub(crate) struct SpectrumState {
    pub(crate) chars: [char; 16],
    pub(crate) bg: [bool; 16],
    pub(crate) bg_accum: [bool; 16],
    pub(crate) last_update: Option<Instant>,
    pub(crate) last_bg_update: Option<Instant>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum NudgeMode { Jump, Warp }

#[allow(dead_code)]
pub(crate) enum NotificationStyle { Info, Warning, Error, Success }

pub(crate) struct Notification {
    pub(crate) message: String,
    pub(crate) style:   NotificationStyle,
    pub(crate) expires: Instant,
}

pub(crate) const TAG_FIELD_LABELS: &[&str] = &[
    " Artist", "  Title", "  Album", "   Year", "  Track", "  Genre", "Comment",
];
pub(crate) const TAG_EDITOR_MAX_WIDTH: u16 = 64;
pub(crate) const TAG_EDITOR_MIN_WIDTH: u16 = 36; // 2 borders + 9 label + at least ~25 chars of text

pub(crate) struct TagEditorState {
    pub(crate) fields:          Vec<(String, usize)>,
    pub(crate) active_field:    usize,
    pub(crate) current_stem:    String,
    pub(crate) extension:       String,
    pub(crate) collision_error: Option<String>,
}

impl TagEditorState {
    pub(crate) fn active_field_mut(&mut self) -> (&mut String, &mut usize) {
        let (val, cur) = &mut self.fields[self.active_field];
        (val, cur)
    }
    pub(crate) fn preview(&self) -> String {
        let a = self.fields[0].0.trim();
        let t = self.fields[1].0.trim();
        format!("{t} - {a}")
    }
}

pub(crate) struct Deck {
    pub(crate) filename: String,
    pub(crate) path:     std::path::PathBuf,
    pub(crate) track_name: String,
    pub(crate) total_duration: f64,
    pub(crate) volume: f32,
    pub(crate) filter_offset: i32,
    pub(crate) nudge: i8,
    pub(crate) nudge_mode: NudgeMode,
    pub(crate) metronome_mode: bool,
    pub(crate) last_metro_beat: Option<i128>,
    pub(crate) active_notification: Option<Notification>,
    pub(crate) cue_sample: Option<usize>,
    pub(crate) rename_hint: Option<String>,
    pub(crate) rename_offer_started: Option<Instant>,
    pub(crate) rename_accepted: Option<String>,
    pub(crate) tag_editor: Option<TagEditorState>,

    pub(crate) audio: DeckAudio,
    pub(crate) tempo: TempoState,
    pub(crate) tap: TapState,
    pub(crate) display: DisplayState,
    pub(crate) spectrum: SpectrumState,
}

impl Deck {
    pub(crate) fn new(
        filename: String,
        path: std::path::PathBuf,
        track_name: String,
        total_duration: f64,
        rename_hint: Option<String>,
        audio: DeckAudio,
        bpm_rx: std::sync::mpsc::Receiver<(String, f32, i64, bool)>,
    ) -> Self {
        Deck {
            filename,
            path,
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
            rename_offer_started: rename_hint.as_ref().map(|_| Instant::now()),
            rename_hint,
            rename_accepted: None,
            tag_editor: None,
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
                vinyl_speed: 1.0,
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
                overview_rect: Rect::default(),
                last_bar_cols: Vec::new(),
                last_bar_times: Vec::new(),
                palette: PALETTE_SCHEMES[0].1, // corrected to slot-specific palette on load
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

    pub(crate) fn rename_offer_active(&self) -> bool {
        self.rename_offer_started.is_some()
            && self.rename_hint.is_some()
            && self.tempo.pending_bpm.is_none()
            && self.active_notification.is_none()
            && self.rename_accepted.is_none()
    }
}

/// Compute BPM and phase offset from a list of tap times (track position in seconds).
/// BPM = linear regression slope across all taps (beat index vs time), which converges
/// as taps accumulate — later taps add leverage and reduce variance.
/// Outlier taps (residual > half a beat period) are dropped before the final regression.
/// Offset = mean residual anchored to the first tap, avoiding phase drift from imprecise period.
pub(crate) fn compute_tap_bpm_offset(tap_times: &[f64]) -> (f32, i64) {
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

pub(crate) fn linear_regression_period(tap_times: &[f64]) -> f64 {
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

/// After a BPM change, re-anchor `offset_ms` so the beat grid stays aligned to
/// the cue position. With no cue set this is a no-op.
pub(crate) fn anchor_beat_grid_to_cue(d: &mut Deck) {
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
pub(crate) fn apply_offset_step(d: &mut Deck, delta_ms: i64) {
    d.tempo.offset_ms += delta_ms;
    let period = (60_000.0 / d.tempo.base_bpm as f64 / 10.0).round() as i64 * 10;
    d.tempo.offset_ms = d.tempo.offset_ms.rem_euclid(period);
    if d.audio.player.is_paused() {
        let delta_samp = delta_ms as f64 / 1000.0 * d.audio.sample_rate as f64;
        d.display.smooth_display_samp = (d.display.smooth_display_samp + delta_samp).max(0.0);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
    }
}

/// Compute 16 braille spectrum characters from mono samples at `pos`.
/// Uses the Goertzel algorithm on 32 log-spaced bins, 20 Hz – 20 kHz.
pub(crate) fn compute_spectrum(mono: &[f32], pos: usize, sample_rate: u32, filter_offset: i32) -> ([char; 16], [bool; 16]) {
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
///
/// While playing: swallow jumps that would hit either end-stop (preserves beat alignment).
/// Forward guard keeps at least one jump-size from the end.
/// While paused: clamp to boundaries so the user can navigate to the start or end deliberately.
pub(crate) fn do_jump(seek_handle: &SeekHandle, player: &rodio::Player, bpm: f32, track_end: f64, beats: i32) {
    let jump    = beats.unsigned_abs() as f64 * 60.0 / bpm as f64;
    let current = seek_handle.current_pos().as_secs_f64();
    let playing = !player.is_paused();
    if beats < 0 {
        let target = current - jump;
        if playing && target < 0.0 { return; }
        let clamped = target.max(0.0);
        if playing { seek_handle.seek_to(clamped); } else { seek_handle.seek_direct(clamped); }
    } else {
        let target = current + jump;
        if playing && target + jump > track_end { return; }
        let clamped = target.min(track_end);
        if playing { seek_handle.seek_to(clamped); } else { seek_handle.seek_direct(clamped); }
    }
}

/// Seek by a fixed number of seconds (vinyl mode jump).
pub(crate) fn do_time_jump(seek_handle: &SeekHandle, player: &Player, track_end: f64, secs: f64) {
    let current = seek_handle.current_pos().as_secs_f64();
    let playing  = !player.is_paused();
    if secs < 0.0 {
        let target = (current + secs).max(0.0);
        if playing { seek_handle.seek_to(target); } else { seek_handle.seek_direct(target); }
    } else {
        let target = current + secs;
        if playing && target + secs > track_end { return; }
        let clamped = target.min(track_end);
        if playing { seek_handle.seek_to(clamped); } else { seek_handle.seek_direct(clamped); }
    }
}

// Suppress the unused import warning — FilterSource is used in main.rs via build_deck
// which constructs it, but it's imported here for the type to be in scope for DeckAudio.
// The actual use of FilterSource is in main.rs::build_deck.
#[allow(unused_imports)]
pub(crate) use crate::audio::FilterSource as _FilterSourceReexport;
