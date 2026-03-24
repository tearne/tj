use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result as EyreResult;
use rodio::Source;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub(crate) const OVERVIEW_RESOLUTION: usize = 4000;
pub(crate) const FADE_SAMPLES: i64 = 256; // ~5.8ms at 44100 Hz — fade-out then fade-in around each seek

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
    /// Counts down from FADE_SAMPLES to 0 after a state reset; output is scaled
    /// by an ascending ramp so any IIR settling transient is inaudible.
    pub(crate) output_fade_remaining: u32,
    pub(crate) last_offset: i32,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
    // Per-channel biquad history
    pub(crate) x1: Vec<f32>, pub(crate) x2: Vec<f32>,
    pub(crate) y1: Vec<f32>, pub(crate) y2: Vec<f32>,
    // Normalised coefficients (a0 = 1)
    pub(crate) b0: f32, pub(crate) b1: f32, pub(crate) b2: f32, pub(crate) a1: f32, pub(crate) a2: f32,
    // Which channel slot we are about to emit
    pub(crate) ch_idx: usize,
}

impl<S: Source<Item = f32>> FilterSource<S> {
    pub(crate) fn new(inner: S, filter_offset: Arc<std::sync::atomic::AtomicI32>, filter_state_reset: Arc<AtomicBool>) -> Self {
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
pub(crate) struct SeekHandle {
    pub(crate) samples: Arc<Vec<f32>>,
    pub(crate) position: Arc<AtomicUsize>,
    pub(crate) fade_remaining: Arc<AtomicI64>,
    pub(crate) fade_len: Arc<AtomicI64>,
    pub(crate) pending_target: Arc<AtomicUsize>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

impl SeekHandle {
    /// Current playback position derived from the atomic sample counter.
    pub(crate) fn current_pos(&self) -> Duration {
        let pos = self.position.load(Ordering::Relaxed);
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
        self.fade_len.store(FADE_SAMPLES, Ordering::SeqCst);
        self.pending_target.store(target_sample, Ordering::SeqCst);
        self.fade_remaining.store(-FADE_SAMPLES, Ordering::SeqCst);
    }

    /// Seek to `target_secs` directly, without a fade. Used when paused — the audio
    /// thread is not calling next(), so the fade-based seek would never execute.

    pub(crate) fn seek_direct(&self, target_secs: f64) {
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
    pub(crate) fn set_position(&self, target_secs: f64) {
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
