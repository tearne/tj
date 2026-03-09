# Design: Needle Drop
**Status: Draft**

## Approach

1. Enable mouse capture at terminal setup (`EnableMouseCapture`) and disable it on teardown (`DisableMouseCapture`).
2. In the event loop, handle `Event::Mouse` alongside `Event::Key`. On a left-button `Down` event, check whether the click row falls within the Overview panel (`chunks[1]`). If so, convert the click column to a track position, find the nearest bar marker at or to the left, and seek to it using `seek_to` (playing) or `seek_direct` (paused).
3. The Overview occupies `chunks[1]`. Its area is available after the `terminal.draw` call via the layout. To make the area accessible in the event handler, store it in a variable outside the draw closure (ratatui areas are `Rect` values, cheap to copy).

### Click-to-sample conversion
```
click_col ∈ [0, ow)
track_secs = (click_col / ow) * total_duration.as_secs_f64()
```

### Nearest bar marker to the left
Iterate `bar_cols` (already computed each frame) and find the largest value ≤ the click column. Convert that column back to seconds the same way. If no bar marker is at or left of the click, seek to 0.

## Tasks
1. ✓ Impl: Enable/disable mouse capture in terminal setup and teardown
2. ✓ Impl: Store Overview `Rect` outside the draw closure
3. ✓ Impl: Handle `Event::Mouse` — left-button click in Overview area → seek to nearest bar marker at or left of click
4. Verify: clicking Overview jumps to nearest bar marker; works while paused and playing
5. Process: confirm ready to archive
