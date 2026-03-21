# Design: Shared Tick Row
**Status: Approved**

## Approach

The change touches three things: the renderer row count, the `render_detail_waveform` function, and the call sites for both decks.

### Renderer rows

Currently: `shared_renderer.rows = h.saturating_sub(2)` — subtracts 2 for the top and bottom tick rows.

After: `shared_renderer.rows = h.saturating_sub(1)` — subtracts 1 (only the shared tick row at the bottom of deck A; deck B gains that extra waveform row too for symmetry, and the buffer is sized the same for both).

### `render_detail_waveform` changes

Add a parameter `shared_tick: Option<&[u8]>`:

- `Some(tick)` → deck A behaviour: render `h-1` waveform rows followed by 1 shared tick row. The `tick` slice is the pre-computed OR of both decks' tick patterns.
- `None` → deck B behaviour: render `h-1` waveform rows only (last row of the area left blank by the Paragraph widget).

The `is_tick_row` logic currently marks `r == 0` and `r + 1 == h` as tick rows. After:
- Neither row 0 nor any waveform row is a tick row — remove the top-tick concept entirely.
- The waveform buffer row mapping changes from `buf_r = r - 1` (skipping old top tick) to `buf_r = r`.

### Shared tick computation

Before calling `render_detail_waveform` for deck A, compute the tick display for both decks separately using the existing per-deck logic (extracted into a helper `compute_tick_display`), then OR them together byte-wise into a single `shared_tick: Vec<u8>`. Pass this to deck A's render call; pass `None` to deck B's.

If deck B is not loaded, the shared tick shows only deck A's ticks (the OR is just deck A's vector).

### Cue marker

Currently the cue marker is rendered on the tick rows (top and bottom). After the change, the cue marker sits on the waveform rows at `r == 0` (top edge of the waveform) and `r + 1 == detail_panel_rows - 1` (bottom waveform row, one above the shared tick). The `is_tick_row` condition used for the green cue colour is replaced by an `is_edge_row` check against these new boundaries.

### `detail_height` semantics and `DET_MIN`

`detail_height` already means the total rows of each deck's detail area. This doesn't change. `DET_MIN` stays at 3: with 1 shared tick row at the bottom of deck A, deck A has 2 waveform rows minimum; deck B also has 2 waveform rows minimum (rows 0 and 1 of its area, last row blank).

## Tasks

1. ✓ Impl: Extract `compute_tick_display` helper from `render_detail_waveform`
2. ✓ Impl: Update `shared_renderer.rows` from `h - 2` to `h - 1`
3. ✓ Impl: Add `shared_tick: Option<&[u8]>` parameter to `render_detail_waveform`; update waveform row logic (remove top tick, remap `buf_r = r`, add shared tick as final row for deck A)
4. ✓ Impl: Update call sites — compute shared tick before deck A render, pass `None` for deck B
5. ✓ Impl: Update cue marker edge rows to match new layout
6. Process: Confirm ready to archive
