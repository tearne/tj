# Design: Smooth Scroll Cap
**Status: Approved**

## Approach
On line 381 of `src/main.rs`, `elapsed` is computed from wall-clock time and used directly to advance `smooth_display_samp`. Cap it at `2.0 / FPS_LEVELS[fps_idx] as f64` (2× the current frame period) before applying. The existing gentle drift correction (10% pull per frame) handles any lag that accumulates as a result.

## Tasks
1. Impl: Cap `elapsed` at 2× the current frame period before advancing `smooth_display_samp`
2. Verify: Observe that the periodic scroll jump no longer occurs
3. Process: confirm ready to archive
