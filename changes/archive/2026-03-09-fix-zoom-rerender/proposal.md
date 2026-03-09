# Proposal: Fix Waveform Rerender at Maximum Zoom
**Status: Draft**

## Intent
At maximum zoom (1s), the Detail view waveform abruptly jumps to a significantly different position approximately every 8 seconds during playback.

Root cause: the smooth display position elapsed cap (`col_secs × 0.75`) is smaller than the minimum poll duration (8ms) at tight zoom. At 1s zoom with 100 columns, `col_secs = 10ms`, so the cap is 7.5ms. Each frame, `smooth_display_samp` advances only 7.5ms worth of samples while the frame period is 8ms, accumulating ~22 samples of deficit per frame. After ~1000 frames (8 seconds) the deficit exceeds the 0.5s large-drift snap threshold and `smooth_display_samp` jumps forward — producing the visible abrupt rerender.

## Specification Deltas

### MODIFIED
- **Rendering — Smooth display position**: The elapsed cap that prevents single-frame jumps must be large enough that it is never smaller than the frame period. A zoom-relative cap that falls below the minimum frame period causes systematic drift and periodic snapping. (Implementation detail: the cap should be expressed as an absolute bound rather than a fraction of `col_secs`.)
