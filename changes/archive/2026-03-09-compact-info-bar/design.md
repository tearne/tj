# Design: Compact Info Bar
**Status: Approved**

## Approach

### Layout changes
Current six-chunk layout:
```
[0] BPM + offset          (1 row)
[1] Overview              (5 rows)
[2] Detail + blank        (Min 0)
[3] Beat indicator        (1 row)
[4] Status + time         (1 row)
[5] Key hints             (1 row)
```
New four-chunk layout:
```
[0] Info bar              (1 row)
[1] Overview              (5 rows)
[2] Detail + blank        (Min 0)  ← gains 3 rows
```
Chunks [3], [4], [5] are removed. The detail area grows by 3 rows.

### Info bar format
A single `Line` of `Span`s on one row:
```
▶  128 bpm  +0ms  ×16  4s  [?]
⏸  128 bpm  +0ms  ×16  4s  [?]
```
Fields (left to right): play/pause icon (`▶`/`⏸`), BPM, phase offset, beat jump unit (`×N`), zoom level, `[?]`. During BPM analysis the BPM field shows the spinner as before. Volume will be added to the bar when the track-volume change is implemented.

The bar is a single `Line` — ratatui wraps it naturally if the terminal is too narrow.

### Beat flash
The beat flash window and timing are unchanged. Instead of colouring a separate panel, the BPM span in the info bar is styled `fg(Yellow).bg(Color::Rgb(60,50,0))` (dim amber background) during the flash window, reverting to normal outside it. This gives a soft glow rather than a bright flash.

### Help popup
A `bool help_open` flag controls visibility. Pressing `?` toggles it; any other key dismisses it (and is otherwise ignored while open). The popup is rendered as a modal overlay using ratatui's `Clear` widget over a centred `Rect`, with a bordered `Block` and a `Paragraph` listing all bindings. The popup is rendered last so it appears above everything else.

Helper to compute a centred rect of fixed size within the terminal area:
```rust
fn centred_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect { x, y, width: width.min(area.width), height: height.min(area.height) }
}
```

## Tasks
1. ✓ Impl: Remove chunks [3]/[4]/[5]; add info bar at chunks[0]; update all chunk index references
2. ✓ Impl: Compose info bar Line with play/pause icon, BPM, offset, jump unit, zoom, `[?]`
3. ✓ Impl: Beat flash — style BPM span with dim amber background during flash window
4. ✓ Impl: Help popup — `help_open` flag, `?` key handler, Clear + Block + Paragraph overlay
5. ✓ Verify: info bar displays all fields correctly; beat flash visible as soft glow on BPM; help popup opens/closes; detail area is taller
6. ✓ Process: confirm ready to archive
