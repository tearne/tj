# Beat Jump End-Stop Guard
**Type**: Fix
**Status**: Approved

## Problem

While playing, beat jumps that overshoot the track boundaries break beat alignment:

- **Backward past start**: currently clamps to position 0, which lands off-beat.
- **Forward near end**: currently a no-op only when past the end, allowing jumps that land so close to the end that the track ends before the next beat cycle.

While paused, boundary clamping is fine — navigation to the start or end is intentional.

There is also a secondary bug: `total_duration` uses integer-division (`len / rate`), truncating up to ~1 second from the true track end.

## Behaviour

**While playing:**
- Backward: swallow the jump if `target < 0` (no-op; don't clamp).
- Forward: swallow the jump if `target + jump_secs > track_end` — i.e. only allow the jump if there is at least one full jump-size of track remaining after landing. This keeps the playhead at least one jump away from the end.

**While paused:**
- Backward: clamp to 0 (unchanged).
- Forward: clamp to `track_end` (symmetric with backward; currently a no-op, change to clamp).

**Track end precision:**
- Compute as `mono.len() as f64 / sample_rate as f64` and pass to `do_jump` as `f64`, dropping the `Duration` parameter.

## Log

Implemented as designed. Also fixed a panic in `spectral_color`: `3.0 - f32::EPSILON` rounds back to `3.0` in f32 (ULP at 3.0 is `2 * EPSILON`), causing `seg = 3` and an out-of-bounds access on the 4-element stops array. Fixed by clamping `seg` to `stops.len() - 2` after the floor.
