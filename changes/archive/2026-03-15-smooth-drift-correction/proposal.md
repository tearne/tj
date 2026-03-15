# Proposal: Smooth Drift Correction
**Status: Complete**

## Intent

Replace the threshold-based snap correction for `smooth_display_samp` with a continuous slew correction that eliminates periodic visible jumps in the waveform scroll.

## Background

`smooth_display_samp` is a floating-point position that advances each frame by `elapsed * sample_rate * speed`. Because `elapsed` is never perfectly consistent (frame timing jitter, variable terminal write time), small errors accumulate over time. The previous correction fired when drift exceeded 0.5 seconds:

```rust
if drift.abs() > sample_rate as f64 * 0.5 {
    let half_col = col_samp / 2.0;
    smooth_display_samp = (pos_samp as f64 / half_col).round() * half_col;
}
```

Even snapping to the nearest half-column, this produced a visible periodic jump — uniform in character because the drift accumulated at a near-constant rate and always corrected by a similar amount.

With a fast terminal (Alacritty), the timing improvements from the smooth-detail-scroll work reduced other sources of jitter enough that this snap became the dominant visible artifact.

## Fix — Continuous Slew Correction

A small fractional correction is applied toward the true audio position every frame:

```rust
smooth_display_samp += elapsed * sample_rate * speed;
let drift = smooth_display_samp - pos_samp as f64;
if drift.abs() > sample_rate as f64 * 0.5 {
    // Backstop: snap for seeks / startup
} else if !player.is_paused() {
    smooth_display_samp -= drift * 0.05;
}
```

At 5% per frame and ~60fps, drift is halved roughly every 14 frames (~230ms). Each individual correction is well below one braille dot and visually imperceptible. The backstop snap is retained for large discontinuities (seeks, beat jumps) where the slew rate would be too slow.

Applied to both the active and inactive deck drift correction paths.

## Result

Very successful. Periodic jumps eliminated. Combined with the Alacritty discovery (WezTerm's rendering pipeline was the primary throughput bottleneck), the waveform scroll is now smooth.
