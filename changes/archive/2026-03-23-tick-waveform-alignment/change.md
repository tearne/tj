# Tick–Waveform Alignment
**Type**: Fix
**Status**: Approved

## Problem

Tick marks in the shared tick row wobble slightly against the waveform as the
playhead moves.

The waveform viewport is snapped to the nearest half-column boundary so the
braille grid stays locked to discrete buffer positions. The tick row is computed
on the draw thread using the exact `display_pos_samp`, which moves continuously.
The two use different reference positions, so tick marks drift by up to half a
column relative to the waveform peaks they should align with.

(Cue markers deliberately use the exact position for a different reason — the
quantised value there causes its own wobble. Tick marks need the opposite
treatment: locked to the waveform grid.)

## Fix

Use the same half-column-quantized view start for the tick row:

```rust
let mvs = if buf.samples_per_col > 0 {
    let half_col = buf.samples_per_col as f64 / 2.0;
    let delta    = pos as i64 - buf.anchor_sample as i64;
    let delta_half = (delta as f64 / half_col).round() as i64;
    let delta_cols = delta_half.div_euclid(2);
    buf.anchor_sample as f64
        + (delta_cols - centre_col as i64) as f64 * buf.samples_per_col as f64
} else { 0.0 };
```

This locks `mvs` to the same grid as `viewport_start`, so tick positions track
the waveform without drift.

## Log

Initial fix: quantised `mvs` to the nearest half-column boundary (using
`delta_half × half_col` instead of `delta_cols × spc`). This aligned ticks to
the waveform's whole-column grid but left half-column wobble at low zoom because
the sub-column component was missing.

Second fix: moved tick rendering into the background pipeline. Each
`BrailleBuffer` now carries a `tick: Vec<u8>` row computed alongside the
waveform at the same anchor and `samples_per_col`. At draw time,
`extract_tick_viewport` extracts the viewport slice with a semantic half-column
shift (flipping `0x47`↔`0xB8` and shifting column indices) rather than the
braille bit-manipulation used for waveform rows — this is required because raw
braille bits would be scrambled by `shift_braille_half`. Alignment is guaranteed
by construction; no per-frame position calculation is needed.

`BrailleBuffer` gained `tick: Vec<u8>`. `SharedDetailRenderer` gained `bpm_a/b`
and `offset_ms_a/b` atomics, updated each frame from deck state, so the
background thread has the tempo data it needs. `store_tempo` added to
`SharedDetailRenderer`. The draw-thread `tick_for` closure removed; replaced by
`extract_tick_viewport`. `SPEC/waveforms.md` updated to reflect the new model.
