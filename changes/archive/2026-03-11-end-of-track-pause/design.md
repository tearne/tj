# Design: End-of-Track Pause
**Status: Approved**

## Approach

Replace the `player.empty()` exit with a pause: call `player.pause()` and clamp `smooth_display_samp` to the last sample position. The player view remains open and the render loop continues normally.

`player.empty()` becomes true when the rodio sink drains. At that point we pause — rodio won't play further audio when paused, so no special handling is needed for the empty sink state.

## Tasks

1. ✓ **Impl**: Replace `if player.empty() { return Ok(None); }` with a pause + clamp to end.
2. ✓ **Verify**: Track plays to end, stops, UI remains open; `Space+Z` / seek still work from end position.
3. **Process**: Archive
