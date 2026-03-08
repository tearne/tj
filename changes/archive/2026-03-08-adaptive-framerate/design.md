# Design: Adaptive Framerate
**Status: Approved**

## Approach
Replace the fixed `poll_ms = 1000 / FPS_LEVELS[fps_idx]` with the original column-duration approach: `poll_ms = col_samp_ms`, clamped to 8–200ms (120 fps max, 5 fps min). Remove all manual FPS control (constant, variable, key handlers, status bar entry). Remove the elapsed cap introduced during the smooth-scroll experiments.

Restore the small drift correction, but set rate to `1.0` (immediate snap) so any firing is visibly obvious — this lets us empirically determine whether the correction is ever needed in practice.

## Tasks
1. ✓ Impl: Remove `FPS_LEVELS`, `fps_idx`, elapsed cap; restore adaptive `poll_ms`; remove `f`/`F` handlers and fps from status bar
2. ✓ Impl: Restore small drift correction at rate `1.0`
3. ✓ Verify: Observe whether scrolling is regular; observe whether drift correction ever snaps
4. Process: confirm ready to archive
