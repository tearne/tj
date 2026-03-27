# Limiter Spike — Findings

## Approach

**Agreed scope**: hard clip at 0 dBFS, waveform columns that would clip shown in a different colour.

### 1. Hard clip — `src/audio/mod.rs`

`FilterSource::next()` applies gain at line 297–298:

```rust
let gain = f32::from_bits(self.gain.load(Ordering::Relaxed));
let filtered = filtered * gain;
```

Add one line after:

```rust
let filtered = filtered.clamp(-1.0, 1.0);
```

This sits before the PFL routing block, so the clip applies uniformly to both the main mix and the PFL monitor.

### 2. Clip indicator in the detail waveform — `src/render/mod.rs`

`BrailleBuffer` is built by the background thread from pre-gain raw audio. It currently stores `bass_ratio: Vec<f32>` (clamped peaks) per column. Add a parallel field:

```rust
pub(crate) abs_peak: Vec<f32>, // unclamped max absolute peak per column
```

Populated by a new helper alongside `spectral_for_slot` — iterates the same column windows but returns `max(|min|, |max|)` without clamping.

At render time in `render_detail_waveform`, `deck.gain_db` is available. Compute:

```rust
let gain = 10f32.powf(deck.gain_db as f32 / 20.0);
let clipped = buf.abs_peak.get(buf_col).map_or(false, |&p| p * gain > 1.0);
```

If `clipped`, substitute an orange-red colour instead of the spectral colour for that column.

### 3. Overview waveform

The overview uses the same `WaveformData.peaks` (which are also pre-gain). Could apply the same clip colour there, but leaving that for a follow-up — the detail view is the primary indicator.

## Open questions resolved

| Question | Decision |
|---|---|
| Ceiling | Fixed 0 dBFS |
| Knee | Hard clip |
| UI feedback | Clip colour in the detail waveform |
| Ceiling configurable | No — revisit if needed |

## Risk

None identified. The clamp is a trivial addition to the audio hot path. The `abs_peak` field adds one `Vec<f32>` per buffer (same allocation pattern as `bass_ratio`).
