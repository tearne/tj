# Design: Empty Deck Placeholder UI
**Status: Approved** *(retrospective)*

## Approach

Implemented as part of the multi-deck refactor. Four helper functions cover the empty-slot placeholders for each section:

- `notification_line_empty()` — dim deck label + "no track — press z to open the file browser"
- `info_line_empty(bar_width)` — `⏸  ---  +0ms` in dim style; level and filter widgets omitted
- `overview_empty(rect)` — calls `render_braille` with zero-amplitude peaks and 120 BPM tick marks (same path as the active overview renderer, keeping the placeholder visually consistent)
- `render_detail_empty(frame, area, display_cfg)` — renders a braille buffer of all-zero peaks, producing a faint vertical centre-line at the playhead column; all other columns blank

Layout constraints were changed so detail sections use `Constraint::Length(detail_height)` derived from whichever deck is loaded (defaulting to 8), rather than collapsing to zero when a deck slot is empty.

The startup behaviour was also changed: `main()` no longer opens the file browser when no path argument is given. Instead it passes `None` as the initial deck to `tui_loop`, which displays empty-deck panels for Deck A with a 60-second global notification prompting the user to press `z`.

## Tasks

1. ✓ **Impl**: `notification_line_empty`, `info_line_empty`, `overview_empty`, `render_detail_empty` — four placeholder helpers
2. ✓ **Impl**: Change startup behaviour — `tui_loop` accepts `Option<Deck>`; no auto-browser on launch; 60-second global notification
3. ✓ **Impl**: Update layout constraints so all deck sections render at fixed height regardless of load state
4. ✓ **Process**: Archive
