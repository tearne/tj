# Design: FPS Control
**Status: Approved**

## Approach
Add a `FPS_LEVELS: &[u64]` constant with discrete fps values. Track the current level with a `fps_idx` variable. Replace the existing auto-calculated `poll_ms` (which targeted one column per frame) with `1000 / FPS_LEVELS[fps_idx]`. Add `f`/`F` key handlers to cycle the index. Display the current fps in the key hints line.

## Tasks
1. ✓ Impl: Add `FPS_LEVELS` constant and `fps_idx` variable (default index 3 → 30 fps)
2. ✓ Impl: Replace `poll_ms` auto-calculation with `1000 / FPS_LEVELS[fps_idx]`
3. ✓ Impl: Add `f`/`F` key handlers to cycle fps down/up
4. ✓ Impl: Show `f/F: fps(N)` in the key hints status bar
5. Process: confirm ready to archive
