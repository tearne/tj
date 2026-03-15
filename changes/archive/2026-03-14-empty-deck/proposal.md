# Proposal: Empty Deck Placeholder UI
**Status: Approved**

## Intent

When a deck slot has no track loaded, the UI should still render a full-height, recognisable deck panel — not a collapsed or blank area. This makes the two-deck layout legible at startup and before Deck B is loaded.

## Specification Deltas

### ADDED

- **Empty deck panel**: when a deck slot contains no track, all its sections render at full height with placeholder content:
  - **Notification bar**: dim label ("A" or "B") + "no track — press z to open the file browser"
  - **Info bar**: `⏸  ---  +0ms` in dim style, with the level and filter widgets omitted
  - **Overview**: a faint flat horizontal line at the vertical midpoint
  - **Detail waveform**: a faint vertical centre-line (the playhead column) spanning the full height, same position as the active playhead indicator would be; all other columns blank

### MODIFIED

- **Layout constraints**: detail sections always use a fixed height (the active deck's `detail_height` as the reference, or 8 if neither deck is loaded). Empty deck sections no longer collapse to zero.
