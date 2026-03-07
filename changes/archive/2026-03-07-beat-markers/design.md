# Design: Beat Markers on Waveform
**Status: Ready for Review**

## Approach

Both marker sets are computed each frame from `bpm`, `offset_ms`, and the current playback position. No new state is needed.

### Overview — bar ticks
- Bar period: `4 * 60.0 / bpm` seconds.
- First bar offset: `offset_ms / 1000.0` seconds (may be negative; walk forward with `rem_euclid` to find first bar in range).
- For each bar time in `[0, total_duration]`, convert to an x coordinate: `(bar_time / total_duration) * width`.
- Draw a short tick at the very bottom of the canvas (e.g. a 1-unit-tall line at `y = -1.0` to `-0.85`). Use a muted colour (e.g. `Color::DarkGray`) so it doesn't compete with the waveform.

### Detail — beat lines
- Beat period: `60.0 / bpm` seconds.
- Visible window: `[pos - zoom_secs/2, pos + zoom_secs/2]`.
- Walk beat positions in that range and convert each to a column: `(beat_time - window_start) / zoom_secs * width`.
- Draw a full-height vertical line at each beat column. Use a dim colour (e.g. `Color::DarkGray`) so the cyan waveform remains dominant.

Both computations are pure functions — no structs needed, just helper fns called inside the `Canvas::paint` closure.

## Tasks
1. ✓ Impl: add `draw_bar_ticks` helper — takes `(ctx, bpm, offset_ms, total_secs, width)`, draws bar ticks on overview canvas
2. ✓ Impl: add `draw_beat_lines` helper — takes `(ctx, bpm, offset_ms, pos_secs, zoom_secs, width)`, draws beat lines on detail canvas
3. ✓ Impl: call both helpers from `tui_loop` inside the relevant `Canvas::paint` closures
4. ✓ Verify: build and smoke-test — markers visible, shift with offset adjustment, correct density at various BPMs and zoom levels
5. ✓ Process: confirm ready to archive
