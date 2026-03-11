# Proposal: HPF / LPF Filter
**Status: Ready for Review**

## Intent
Provide a real-time high-pass / low-pass filter on the playback output, useful for mixing, cueing, and beatmatching workflows. A single control sweeps from HPF (cutting lows) through flat (no filtering) to LPF (cutting highs). A Space-modifier chord snaps the filter back to flat instantly.

## Specification Deltas

### ADDED
- A single filter parameter `filter_offset: i32` (range −10 to +10, default 0) controls the active filter:
  - `0` — flat (no filtering, filter bypassed).
  - `−1` to `−10` — low-pass filter; more negative = lower cutoff frequency.
  - `+1` to `+10` — high-pass filter; more positive = higher cutoff frequency.
- `[` decreases `filter_offset` by 1 (toward LPF); `]` increases it by 1 (toward HPF). Both clamp at ±10.
- `Space+[` or `Space+]` snaps `filter_offset` to 0 (flat) immediately.
- The filter is a second-order Butterworth IIR filter applied to the audio output stream in real time, recomputed whenever `filter_offset` changes.
- Cutoff frequencies are mapped from the offset steps on a logarithmic scale, covering roughly 40 Hz–18 kHz across the full range. The exact mapping is an implementation detail.
- The filter state is not persisted between sessions; it always initialises to flat.
- The info bar shows a filter indicator when `filter_offset ≠ 0` (e.g. `lpf:3` or `hpf:5`).

### MODIFIED
- `[` and `]` are added as player key bindings.
- `Space+[` and `Space+]` are added as player key bindings (snap to flat).
- The info bar gains a filter indicator field.

## Scope
- **In scope**: real-time IIR filter on playback output, single sweep control, instant flat-snap.
- **Out of scope**: filter type selection (shelf, notch, band-pass), resonance/Q control, per-track persistence.
