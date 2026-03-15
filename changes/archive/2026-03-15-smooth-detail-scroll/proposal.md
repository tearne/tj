# Proposal: Smooth Detail Scroll
**Status: Complete**

## Intent

Investigate and reduce choppiness in the detail waveform scroll animation. The rendering already has half-column (one braille dot) sub-pixel precision; the choppiness has other causes that need to be identified and addressed experimentally.

## Background

The detail waveform is rendered from a `BrailleBuffer` computed by a background thread. Each frame, `render_detail_waveform` computes a viewport offset into the buffer based on `display_pos_samp - buf.anchor_sample`. The offset is quantised to the nearest half-column (one braille dot), which is already the finest horizontal resolution available in terminal braille.

Despite this, the scroll can appear choppy. Identified causes:

### Cause A — Drift-snap quantisation

`smooth_display_samp` is a floating-point position that advances each frame. When it drifts too far from the true audio position, it is corrected with a snap:

```rust
(pos_samp as f64 / col_samp_f64).round() * col_samp_f64
```

This snaps to the nearest **full column**, which can produce a jump of up to one full column (two braille dots) in a single frame. Snapping to the nearest **half-column** instead halves the maximum snap distance and keeps the correction within the precision the renderer already handles.

### Cause B — Irregular frame timing

The original loop slept **before** drawing: it throttled to a frame budget, then drew. Any time spent writing to the terminal came *on top of* the frame budget, making actual frame intervals variable. On a slow or bandwidth-constrained terminal this produced irregular inter-frame timing even when the sleep itself was accurate.

### Cause C — Background thread buffer lag

If the background thread's `anchor_sample` is too far behind the current `display_pos_samp`, the viewport offset falls outside the pre-rendered buffer range and the waveform area shows blank content until the buffer catches up. This produces a frame of blank then a frame with content — a visible flicker.

## Experiments

### Experiment 1 — Half-column drift snap (v0.5.19)

Changed the snap formula from `round(pos / col_samp) * col_samp` to `round(pos / half_col_samp) * half_col_samp` for both the active and inactive deck drift correction paths.

**Result:** Marginal improvement. Jumps felt slightly less severe but the primary problem — irregular jerky frame rate — remained.

### Experiment 2 — Decouple frame timing from events (v0.5.20)

Two changes:
- Sleep moved to the **top** of the loop (before draw) so frame timing is not driven by event arrival.
- Event poll loop changed from `if event::poll(frame_dur)?` to `while event::poll(Duration::ZERO)?` to drain all queued events without blocking.

**Result:** Jumps became less frequent but larger when they did occur. User observed that the waveform appeared smoother when made smaller, suggesting terminal bandwidth (write time) as a contributor.

### Experiment 3 — Sleep after draw (v0.5.21)

Moved the sleep to the **end** of the loop (after `terminal.draw()` and event drain). Records `frame_start` at the top of the loop; sleeps for `frame_dur.saturating_sub(frame_start.elapsed())` at the bottom. Variable terminal write time is now automatically absorbed: a slow flush shortens the sleep rather than delaying the next frame start.

**Result:** Slight improvement. All three experiments kept in the final build.

## Conclusion

All three changes are retained as they are each objectively more correct, and together produce a marginal improvement. However, no decisive improvement in perceived smoothness was observed. The primary bottleneck is believed to be **terminal throughput** — the terminal can only render a limited number of character cell changes per second, and a full braille waveform redraw exceeds this at some zoom levels. No further timing-based experiments are expected to help. The next area of investigation is dirty/partial rendering (reducing the number of cells written per frame).

The A/B toggle (`\` key, `[exp]`/`[---]` indicator) was removed at archive time as it is no longer needed.
