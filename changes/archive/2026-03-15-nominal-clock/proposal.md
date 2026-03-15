# Proposal: Nominal Clock for Display Position Advance
**Status: Draft**

## Problem

The detail waveform has a slight regular oscillation during playback. The oscillation is periodic and correlates with the frame rate.

## Root Cause

`smooth_display_samp` is advanced each frame by `elapsed * sample_rate * speed`, where `elapsed` is the measured wall time since the previous frame. `elapsed` is always slightly larger than the target `frame_dur` because `thread::sleep` overshoots — the OS scheduler wakes the thread after *at least* the requested duration, never exactly. On Linux this overshoot is typically 0.5–2ms per frame and is consistent in sign (always positive) and roughly regular in magnitude.

This means `smooth_display_samp` advances slightly faster than the audio every frame. The 5% slew correction then pulls it back every frame. The result is a tight feedback loop driven by a consistent positive error, producing a regular oscillation at roughly the frame rate.

## Fix

Use `frame_dur` (the nominal target frame duration, computed from `col_secs / 2`) instead of `elapsed` for the wall-clock advance:

```rust
deck.display.smooth_display_samp += frame_dur.as_secs_f64() * sample_rate * speed;
```

`frame_dur` is derived from the same sample rate as the audio, so it matches the actual audio advance rate by construction. Sleep overshoot no longer affects the prediction, and the slew correction has no systematic bias to fight — it only needs to handle genuine long-term drift.

The slew correction is retained unchanged as an accuracy safety net for seeks, slow renders, and any real audio/wall-clock divergence.

## Experiment

Toggled live with `\` in 0.5.42 (nominal vs elapsed). Nominal judged to be noticeably better.

## Change

- Replace `elapsed` with `frame_dur.as_secs_f64()` in both the active and inactive deck display advance.
- Remove the `\` toggle and `use_nominal_frame_dur` state.
- Update the comment on that block.
