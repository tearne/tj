# Design: Beat Jump Unit — 1 Beat
**Status: Draft**

## Approach

Add `1` to the front of `BEAT_UNITS` and shift the key bindings: `1`=1 beat, `2`–`7`=4,8,16,32,64,128. Update `DEFAULT_BEAT_UNIT_IDX` to keep 16 beats as the default (now index 3). Update the key hints string.

## Tasks
1. ✓ Impl: Add `1` to `BEAT_UNITS`; update `DEFAULT_BEAT_UNIT_IDX` to 3; update key hints
2. ✓ Bugfix: Paused seek display not updating — `smooth_display_samp` snap condition `drift > 500ms` was never met for small jumps (1-beat at 120 BPM = exactly 500ms, condition was strict `>`). Fix: also snap when paused and `drift > 1 sample`, since there is no audio jitter when paused.
3. ✓ Verify: `1` selects 1-beat jump; `2`–`7` select 4–128; default remains 16 beats; display updates correctly after paused beat jump
4. ✓ Process: confirm ready to archive
