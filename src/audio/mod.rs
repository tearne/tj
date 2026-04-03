use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result as EyreResult;
use rodio::{Player, Source};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub(crate) const OVERVIEW_RESOLUTION: usize = 4000;
pub(crate) const FADE_SAMPLES: i64 = 256; // ~5.8ms at 44100 Hz — fade-out then fade-in around each seek

/// Width of the soft-knee zone below 0 dBFS. The limiter engages gradually over
/// [1.0 - LIMITER_KNEE, 1.0], using a cubic Hermite that enters with slope 1 and
/// asymptotes to the ceiling with slope 0. Hard clip applies above 1.0.
pub(crate) const LIMITER_KNEE: f32 = 0.3;

/// Apply gain and soft-knee limiting. The knee is a cubic Hermite over
/// [1 - LIMITER_KNEE, 1.0]: slope 1 at entry, slope 0 at the ceiling.
fn apply_gain_and_limit(x: f32, gain: f32) -> f32 {
    let scaled = x * gain;
    let abs_s = scaled.abs();
    let threshold = 1.0 - LIMITER_KNEE;
    if abs_s <= threshold {
        scaled
    } else if abs_s >= 1.0 {
        scaled.signum()
    } else {
        // u ∈ (0, 1): position within the knee zone.
        // Hermite cubic f(u) = u + u² - u³ satisfies f(0)=0, f(1)=1, f'(0)=1, f'(1)=0.
        let u = (abs_s - threshold) / LIMITER_KNEE;
        let f = u + u * u - u * u * u;
        scaled.signum() * (threshold + LIMITER_KNEE * f)
    }
}

/// Log-spaced cutoff frequencies for filter offsets ±1..±16.
/// Index 0 = offset ±1 (near-flat), index 15 = offset ±16 (fully cut).
pub(crate) const FILTER_CUTOFFS_HZ: [f64; 16] = [
    18_000.0, 12_000.0,  8_000.0, 5_300.0,
     3_500.0,  2_350.0,  1_560.0, 1_040.0,
       690.0,    460.0,    306.0,   204.0,
       136.0,     90.0,     60.0,    40.0,
];

/// Compute normalised Butterworth biquad coefficients for a LPF or HPF.
/// Returns `(b0, b1, b2, a1, a2)` with a0 normalised to 1.
pub(crate) fn butterworth_biquad(fc: f64, sample_rate: u32, is_lpf: bool) -> (f32, f32, f32, f32, f32) {
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

// ---------------------------------------------------------------------------
// Waveform data
// ---------------------------------------------------------------------------

pub(crate) struct WaveformData {
    /// Full-track peak envelope at OVERVIEW_RESOLUTION buckets.
    pub(crate) peaks: Vec<(f32, f32)>,
    /// Per-bucket bass ratio in [0,1]: 1.0 = bass-heavy, 0.0 = treble-heavy.
    pub(crate) bass_ratio: Vec<f32>,
    /// Raw mono samples for detail view rendering.
    pub(crate) mono: Arc<Vec<f32>>,
}

impl WaveformData {
    pub(crate) fn compute(mono: Arc<Vec<f32>>, sample_rate: u32) -> Self {
        let n = mono.len();
        let chunk_size = (n / OVERVIEW_RESOLUTION).max(1);
        // First-order IIR low-pass at 250 Hz; bass_ratio = lp_energy / total_energy.
        let alpha = {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * 250.0);
            let dt = 1.0 / sample_rate as f32;
            dt / (rc + dt)
        };
        let mut lp = 0.0f32;
        let (peaks, bass_ratio) = mono
            .chunks(chunk_size)
            .map(|chunk| {
                let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let total_energy: f32 = chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32;
                let lp_energy: f32 = chunk.iter().map(|&s| {
                    lp += alpha * (s - lp);
                    lp * lp
                }).sum::<f32>() / chunk.len() as f32;
                let bass = (lp_energy / (total_energy + 1e-10)).clamp(0.0, 1.0);
                ((min.max(-1.0), max.min(1.0)), bass)
            })
            .unzip();
        Self { peaks, bass_ratio, mono }
    }
}

// ---------------------------------------------------------------------------
// Custom rodio Source + SeekHandle
// ---------------------------------------------------------------------------

pub(crate) struct TrackingSource {
    pub(crate) samples: Arc<Vec<f32>>,
    pub(crate) position: Arc<AtomicUsize>,
    /// Fade state: negative = fading out (counting toward 0), positive = fading in (counting down).
    pub(crate) fade_remaining: Arc<AtomicI64>,
    /// Length of the current fade in samples (FADE_SAMPLES or MICRO_FADE_SAMPLES).
    pub(crate) fade_len: Arc<AtomicI64>,
    /// Pending seek target sample index; usize::MAX means no seek pending.
    pub(crate) pending_target: Arc<AtomicUsize>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

impl TrackingSource {
    pub(crate) fn new(
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

pub(crate) struct FilterSource<S: Source<Item = f32>> {
    pub(crate) inner: S,
    pub(crate) filter_offset: Arc<std::sync::atomic::AtomicI32>,
    pub(crate) filter_state_reset: Arc<AtomicBool>,
    /// Crossfade state: blends from `last_y` to the new output over FADE_SAMPLES on any
    /// discontinuity (engage, disengage, poles change, state reset).
    pub(crate) transition_fade: u32,
    pub(crate) last_y: Vec<f32>,  // per-channel last emitted sample
    pub(crate) last_offset: i32,
    pub(crate) last_poles: u8,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
    // Per-channel biquad history — stage 1
    pub(crate) x1: Vec<f32>, pub(crate) x2: Vec<f32>,
    pub(crate) y1: Vec<f32>, pub(crate) y2: Vec<f32>,
    // Per-channel biquad history — stage 2 (used when filter_poles == 4)
    pub(crate) x1_2: Vec<f32>, pub(crate) x2_2: Vec<f32>,
    pub(crate) y1_2: Vec<f32>, pub(crate) y2_2: Vec<f32>,
    // Normalised coefficients (a0 = 1); shared by both stages
    pub(crate) b0: f32, pub(crate) b1: f32, pub(crate) b2: f32, pub(crate) a1: f32, pub(crate) a2: f32,
    // Which channel slot we are about to emit
    pub(crate) ch_idx: usize,
    /// Number of filter poles: 2 = 12 dB/oct, 4 = 24 dB/oct.
    pub(crate) filter_poles: Arc<AtomicU8>,
    // PFL monitor routing
    pub(crate) pfl_level: Arc<AtomicU8>,
    pub(crate) pfl_active_deck: Arc<AtomicUsize>,
    pub(crate) deck_slot: usize,
    /// Deck volume as f32 bits; used on the right channel when PFL is active (player volume is 1.0 then).
    pub(crate) deck_volume: Arc<AtomicU32>,
    /// Gain trim as f32 bits (linear multiplier); applied pre-fader.
    pub(crate) gain: Arc<AtomicU32>,
}

impl<S: Source<Item = f32>> FilterSource<S> {
    pub(crate) fn new(
        inner: S,
        filter_offset: Arc<std::sync::atomic::AtomicI32>,
        filter_state_reset: Arc<AtomicBool>,
        pfl_level: Arc<AtomicU8>,
        pfl_active_deck: Arc<AtomicUsize>,
        deck_slot: usize,
        deck_volume: Arc<AtomicU32>,
        gain: Arc<AtomicU32>,
        filter_poles: Arc<AtomicU8>,
    ) -> Self {
        let channels = inner.channels().get() as u16;
        let sample_rate = inner.sample_rate().get();
        let n = channels as usize;
        FilterSource {
            inner,
            filter_offset,
            filter_state_reset,
            transition_fade: 0,
            last_y: vec![0.0; n],
            last_offset: 0,
            last_poles: 2,
            channels,
            sample_rate,
            x1: vec![0.0; n], x2: vec![0.0; n],
            y1: vec![0.0; n], y2: vec![0.0; n],
            x1_2: vec![0.0; n], x2_2: vec![0.0; n],
            y1_2: vec![0.0; n], y2_2: vec![0.0; n],
            b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0,
            ch_idx: 0,
            pfl_level,
            pfl_active_deck,
            deck_slot,
            deck_volume,
            gain,
            filter_poles,
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
            for v in self.x1.iter_mut().chain(&mut self.x2).chain(&mut self.y1).chain(&mut self.y2)
                         .chain(self.x1_2.iter_mut()).chain(&mut self.x2_2).chain(&mut self.y1_2).chain(&mut self.y2_2) {
                *v = 0.0;
            }
            for v in self.last_y.iter_mut() { *v = 0.0; }
            self.last_offset = 0;
            self.transition_fade = FADE_SAMPLES as u32;
        }
        let x = self.inner.next()?;
        let offset = self.filter_offset.load(Ordering::Relaxed);
        let poles = self.filter_poles.load(Ordering::Relaxed);

        // Detect pole count change and set up crossfade.
        if poles != self.last_poles {
            if poles >= 4 && self.last_poles < 4 {
                // Pre-fill stage 2 from stage 1 so it starts close to steady state.
                self.x1_2.copy_from_slice(&self.x1);
                self.x2_2.copy_from_slice(&self.x2);
                self.y1_2.copy_from_slice(&self.y1);
                self.y2_2.copy_from_slice(&self.y2);
            } else {
                // Disabling stage 2 — zero its state so it's clean for next enable.
                for v in self.x1_2.iter_mut().chain(&mut self.x2_2)
                             .chain(&mut self.y1_2).chain(&mut self.y2_2) { *v = 0.0; }
            }
            self.last_poles = poles;
            self.transition_fade = FADE_SAMPLES as u32;
        }

        // Detect filter offset change and set up crossfade.
        if offset != self.last_offset {
            if offset == 0 {
                // Disengaging — zero biquad state so re-engage starts cleanly.
                for v in self.x1.iter_mut().chain(&mut self.x2).chain(&mut self.y1).chain(&mut self.y2)
                             .chain(self.x1_2.iter_mut()).chain(&mut self.x2_2)
                             .chain(&mut self.y1_2).chain(&mut self.y2_2) { *v = 0.0; }
            } else {
                self.recompute_coefficients(offset);
            }
            self.last_offset = offset;
            self.transition_fade = FADE_SAMPLES as u32;
        }

        let ch = self.ch_idx;
        self.ch_idx = (ch + 1) % self.channels as usize;

        // Compute current output: filtered when active, raw when bypassed.
        let current = if offset != 0 {
            // Stage 1
            let y = self.b0 * x + self.b1 * self.x1[ch] + self.b2 * self.x2[ch]
                  - self.a1 * self.y1[ch] - self.a2 * self.y2[ch];
            self.x2[ch] = self.x1[ch]; self.x1[ch] = x;
            self.y2[ch] = self.y1[ch]; self.y1[ch] = y;
            // Stage 2 — identical coefficients, active when poles == 4 (24 dB/oct)
            if poles >= 4 {
                let y2 = self.b0 * y + self.b1 * self.x1_2[ch] + self.b2 * self.x2_2[ch]
                       - self.a1 * self.y1_2[ch] - self.a2 * self.y2_2[ch];
                self.x2_2[ch] = self.x1_2[ch]; self.x1_2[ch] = y;
                self.y2_2[ch] = self.y1_2[ch]; self.y1_2[ch] = y2;
                y2
            } else {
                y
            }
        } else {
            x
        };

        // Crossfade from last_y to current on any transition.
        let filtered = if self.transition_fade > 0 {
            let t = self.transition_fade as f32 / FADE_SAMPLES as f32;
            let blended = self.last_y[ch] * t + current * (1.0 - t);
            self.transition_fade = self.transition_fade.saturating_sub(1);
            blended
        } else {
            current
        };
        self.last_y[ch] = filtered;

        let gain = f32::from_bits(self.gain.load(Ordering::Relaxed));
        let filtered = apply_gain_and_limit(filtered, gain);

        // PFL monitor routing (stereo tracks only).
        // When any deck has PFL active, the left channel carries PFL and the main mix is
        // suppressed there; the right channel always carries the main mix at deck volume.
        // player.set_volume() is held at 1.0 for the active PFL deck so that FilterSource
        // can control the right-channel gain independently.
        let pfl_active = self.pfl_active_deck.load(Ordering::Relaxed);
        if self.channels >= 2 && pfl_active != usize::MAX {
            if ch == 0 {
                return if pfl_active == self.deck_slot {
                    let scale = self.pfl_level.load(Ordering::Relaxed) as f32 / 100.0;
                    Some(x * scale)
                } else {
                    Some(0.0)
                };
            } else if pfl_active == self.deck_slot {
                let vol = f32::from_bits(self.deck_volume.load(Ordering::Relaxed));
                return Some(filtered * vol);
            }
        }

        Some(filtered)
    }
}

impl<S: Source<Item = f32>> Source for FilterSource<S> {
    fn current_span_len(&self) -> Option<usize> { self.inner.current_span_len() }
    fn channels(&self) -> NonZero<u16> { NonZero::new(self.channels).unwrap_or(NonZero::new(2).unwrap()) }
    fn sample_rate(&self) -> NonZero<u32> { NonZero::new(self.sample_rate).unwrap_or(NonZero::new(44100).unwrap()) }
    fn total_duration(&self) -> Option<Duration> { None }
}

/// Shared handle for querying playback position and seeking without interrupting the audio thread.
pub(crate) struct SeekHandle {
    pub(crate) samples: Arc<Vec<f32>>,
    pub(crate) position: Arc<AtomicUsize>,
    /// Tracks samples actually emitted by PitchSource; used for display to avoid the 512-sample
    /// batch-read jumps that occur in TrackingSource when pitch is active.
    pub(crate) output_position: Arc<AtomicUsize>,
    pub(crate) fade_remaining: Arc<AtomicI64>,
    pub(crate) fade_len: Arc<AtomicI64>,
    pub(crate) pending_target: Arc<AtomicUsize>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    /// Set by any seek; cleared by PitchSource to flush its internal buffer on discontinuity.
    pub(crate) flush_pitch: Arc<AtomicBool>,
}

impl SeekHandle {
    /// Current playback position derived from the output-position counter.
    pub(crate) fn current_pos(&self) -> Duration {
        let pos = self.output_position.load(Ordering::Relaxed);
        Duration::from_secs_f64(pos as f64 / (self.sample_rate as f64 * self.channels as f64))
    }

    /// Seek to `target_secs`. Triggers a fade-out on the audio thread, which then
    /// atomically jumps to the target and fades back in — no gap, no click.
    /// Find the quietest frame within ±10ms of `target_secs`, to minimise the fade-in transient.
    pub(crate) fn find_quiet_frame(&self, target_secs: f64) -> usize {
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

    pub(crate) fn seek_to(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let best_frame = self.find_quiet_frame(target_secs);
        let target_sample = (best_frame * frame_len).min(self.samples.len());

        // Store the target, then trigger fade-out. The audio thread applies the seek
        // when the fade-out completes and then fades back in.
        self.flush_pitch.store(true, Ordering::Relaxed);
        self.fade_len.store(FADE_SAMPLES, Ordering::SeqCst);
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-FADE_SAMPLES, Ordering::SeqCst);
        // Update output_position immediately so the display snaps to the new position
        // without waiting for the fade to complete.
        self.output_position.store(target_sample, Ordering::SeqCst);
    }

    /// Seek to `target_secs` directly, without a fade. Used when paused — the audio
    /// thread is not calling next(), so the fade-based seek would never execute.

    pub(crate) fn seek_direct(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let best_frame = self.find_quiet_frame(target_secs);
        let target_sample = (best_frame * frame_len).min(self.samples.len());

        // Write position directly and clear any in-progress fade.
        self.flush_pitch.store(true, Ordering::Relaxed);
        self.pending_target.store(usize::MAX, Ordering::SeqCst);
        self.fade_remaining.store(0, Ordering::SeqCst);
        self.position.store(target_sample, Ordering::SeqCst);
        self.output_position.store(target_sample, Ordering::SeqCst);
    }

    /// Move to `target_secs` exactly, without a quiet-frame search or fade.
    /// Used for paused nudge where no click can occur.
    pub(crate) fn set_position(&self, target_secs: f64) {
        let frame_len = self.channels as usize;
        let total_frames = self.samples.len() / frame_len;
        let target_frame = (target_secs * self.sample_rate as f64).round() as usize;
        let target_sample = target_frame.min(total_frames) * frame_len;
        self.pending_target.store(usize::MAX, Ordering::SeqCst);
        self.fade_remaining.store(0, Ordering::SeqCst);
        self.position.store(target_sample, Ordering::SeqCst);
        self.output_position.store(target_sample, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Pitch source — time-domain pitch shift via SoundTouch, tempo unchanged
// ---------------------------------------------------------------------------

pub(crate) struct PitchSource<S: Source<Item = f32>> {
    inner:           S,
    st:              soundtouch::SoundTouch,
    output:          std::collections::VecDeque<f32>,
    pitch_semitones: Arc<std::sync::atomic::AtomicI8>,
    flush_pitch:     Arc<AtomicBool>,
    current_pitch:   i8,
    channels:        u16,
    sample_rate:     u32,
    output_position: Arc<AtomicUsize>,
}

impl<S: Source<Item = f32>> PitchSource<S> {
    pub(crate) fn new(
        inner:           S,
        pitch_semitones: Arc<std::sync::atomic::AtomicI8>,
        flush_pitch:     Arc<AtomicBool>,
        output_position: Arc<AtomicUsize>,
    ) -> Self {
        let channels    = inner.channels().get() as u16;
        let sample_rate = inner.sample_rate().get();
        let mut st = soundtouch::SoundTouch::new();
        st.set_channels(channels as u32);
        st.set_sample_rate(sample_rate);
        PitchSource { inner, st, output: std::collections::VecDeque::new(), pitch_semitones, flush_pitch, current_pitch: 0, channels, sample_rate, output_position }
    }
}

impl<S: Source<Item = f32>> Iterator for PitchSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.flush_pitch.swap(false, Ordering::Relaxed) {
            self.st.clear();
            self.output.clear();
        }

        let new_pitch = self.pitch_semitones.load(Ordering::Relaxed);

        if new_pitch == 0 {
            if self.current_pitch != 0 {
                self.st.clear();
                self.output.clear();
                self.current_pitch = 0;
            }
            let sample = self.inner.next();
            self.output_position.fetch_add(1, Ordering::Relaxed);
            return sample;
        }

        if new_pitch != self.current_pitch {
            self.st.set_pitch_semitones(new_pitch as i32);
            self.output.clear();
            self.current_pitch = new_pitch;
        }

        if let Some(s) = self.output.pop_front() {
            self.output_position.fetch_add(1, Ordering::Relaxed);
            return Some(s);
        }

        // Feed a chunk from the inner source into SoundTouch.
        const CHUNK_FRAMES: usize = 512;
        let n_samples = CHUNK_FRAMES * self.channels as usize;
        let mut chunk = Vec::with_capacity(n_samples);
        for _ in 0..n_samples {
            match self.inner.next() {
                Some(s) => chunk.push(s),
                None    => break,
            }
        }
        if chunk.is_empty() {
            return Some(0.0);
        }
        let n_frames = chunk.len() / self.channels as usize;
        self.st.put_samples(&chunk, n_frames);

        let available = self.st.num_samples().max(0) as usize;
        if available > 0 {
            let mut buf = vec![0.0f32; available * self.channels as usize];
            let received = self.st.receive_samples(&mut buf, available);
            buf.truncate(received * self.channels as usize);
            self.output.extend(buf);
        }

        let sample = self.output.pop_front().or(Some(0.0));
        self.output_position.fetch_add(1, Ordering::Relaxed);
        sample
    }
}

impl<S: Source<Item = f32>> Source for PitchSource<S> {
    fn current_span_len(&self) -> Option<usize> { self.inner.current_span_len() }
    fn channels(&self)       -> NonZero<u16>    { NonZero::new(self.channels).unwrap_or(NonZero::new(2).unwrap()) }
    fn sample_rate(&self)    -> NonZero<u32>    { NonZero::new(self.sample_rate).unwrap_or(NonZero::new(44100).unwrap()) }
    fn total_duration(&self) -> Option<Duration> { None }
}

// ---------------------------------------------------------------------------
// Audio decode
// ---------------------------------------------------------------------------

/// Decode an audio file. Returns (mono_f32, interleaved_f32, sample_rate, channels).
/// Updates `decoded_samples` and `estimated_total` atomics as decode progresses.
pub(crate) fn decode_audio(
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

/// Play a one-shot scrub snippet from the interleaved sample buffer at the given mono position.
/// Injects directly into the mixer so it plays independently of the paused main player.
pub(crate) fn scrub_audio(
    mixer: &rodio::mixer::Mixer,
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    mono_pos: usize,
    mono_len: usize,
) {
    use rodio::buffer::SamplesBuffer;
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

// ---------------------------------------------------------------------------
// Browser preview: streaming decode source + output handle
// ---------------------------------------------------------------------------

/// A streaming rodio `Source` backed by a live symphonia decoder.
/// Constructed at a seek position 20% into the track for instant browser preview.
pub(crate) struct SymphoniaPreviewSource {
    format:      Box<dyn FormatReader>,
    decoder:     Box<dyn Decoder>,
    track_id:    u32,
    sample_rate: u32,
    channels:    u16,
    buffer:      Vec<f32>,
    buffer_pos:  usize,
    sample_buf:  Option<SampleBuffer<f32>>,
    done:        bool,
}

impl SymphoniaPreviewSource {
    pub(crate) fn open(path: &Path) -> EyreResult<Self> {
        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe().format(
            &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
        )?;
        let mut format = probed.format;
        // Extract everything needed from the track before seeking (track borrows format).
        let (track_id, sample_rate, channels, seek_secs, codec_params) = {
            let track = format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| color_eyre::eyre::eyre!("no audio track"))?;
            let track_id    = track.id;
            let sample_rate = track.codec_params.sample_rate
                .ok_or_else(|| color_eyre::eyre::eyre!("no sample rate"))?;
            let channels    = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;
            // 20% of duration; fall back to 30 s if duration is not in the header.
            let seek_secs   = track.codec_params.n_frames
                .map(|n| n as f64 / sample_rate as f64 * 0.20)
                .unwrap_or(30.0);
            let codec_params = track.codec_params.clone();
            (track_id, sample_rate, channels, seek_secs, codec_params)
        };

        let _ = format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time { seconds: seek_secs as u64, frac: seek_secs.fract() },
                track_id: Some(track_id),
            },
        );

        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())?;

        Ok(Self {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
            buffer: Vec::new(),
            buffer_pos: 0,
            sample_buf: None,
            done: false,
        })
    }

    fn fill_buffer(&mut self) -> bool {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(_) => { self.done = true; return false; }
            };
            if packet.track_id() != self.track_id { continue; }
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let buf = self.sample_buf.get_or_insert_with(|| {
                        SampleBuffer::new(decoded.capacity() as u64, *decoded.spec())
                    });
                    buf.copy_interleaved_ref(decoded);
                    self.buffer.clear();
                    self.buffer.extend_from_slice(buf.samples());
                    self.buffer_pos = 0;
                    return true;
                }
                Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
                Err(_) => { self.done = true; return false; }
            }
        }
    }
}

impl Iterator for SymphoniaPreviewSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.done { return None; }
        if self.buffer_pos >= self.buffer.len() {
            if !self.fill_buffer() { return None; }
        }
        let s = self.buffer[self.buffer_pos];
        self.buffer_pos += 1;
        Some(s)
    }
}

impl Source for SymphoniaPreviewSource {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self)    -> NonZero<u16> { NonZero::new(self.channels).unwrap_or(NonZero::new(2).unwrap()) }
    fn sample_rate(&self) -> NonZero<u32> { NonZero::new(self.sample_rate).unwrap_or(NonZero::new(44100).unwrap()) }
    fn total_duration(&self) -> Option<Duration> { None }
}

/// Warm audio output for browser preview. Opened when the browser opens; each
/// `play` call stops any current preview and starts a new one immediately.
pub(crate) struct PreviewOutput {
    player: Player,
}

impl PreviewOutput {
    pub(crate) fn new(mixer: &rodio::mixer::Mixer) -> Self {
        Self { player: Player::connect_new(mixer) }
    }

    pub(crate) fn play(&self, path: &Path) {
        self.player.stop();
        match SymphoniaPreviewSource::open(path) {
            Ok(src) => {
                self.player.append(src);
                self.player.play();
            }
            Err(_) => {}
        }
    }

    pub(crate) fn stop(&self) {
        self.player.stop();
    }
}

impl Drop for PreviewOutput {
    fn drop(&mut self) {
        self.player.stop();
    }
}

/// Synthesise a short 1 kHz click tone and inject it into the mixer.
pub(crate) fn play_click_tone(mixer: &rodio::mixer::Mixer, sample_rate: u32) {
    use rodio::buffer::SamplesBuffer;
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
