# Empty Deck Display
**Type**: Proposal
**Status**: Archived

## Log

- `src/render/mod.rs`: `overview_empty` and `render_detail_empty` replaced with braille dot-mesh fill using U+2895 (dots 1,3,5,8 — seamless checkerboard tile); both take `deck_slot: usize`; all bar-marker, playhead, and tick-row logic removed
- Both decks share the same dark background (`Rgb(11, 11, 15)`); deck A dots are `Rgb(26, 26, 36)`, deck B dots are `Rgb(17, 17, 24)` — same bg, different dot brightness for distinction
- `src/main.rs`: call sites updated to pass `deck_slot` (0 or 1); `vinyl_mode` and `display_cfg` no longer passed to empty render functions

## Problem

The current empty deck rendering sits awkwardly between two honest states: it mimics a waveform (braille flat line, bar markers, playhead column) but lacks the visual elements of a real loaded waveform — no zero line, no full-height playhead, bar markers at a fixed 120 BPM unrelated to any track. It looks like a waveform but isn't one, which is misleading.

## Options

### Option A — Silent waveform

Pre-compute a `BrailleBuffer` of zero-amplitude peaks at startup. Pass it to the normal `render_detail_waveform` and `overview_for_deck` paths with `analysing = true`, display position 0, and a fixed default palette. The empty deck renders identically to a silent loaded track: full-height playhead line, visible zero line (the braille midline encoding), tick marks and bar markers suppressed by `analysing`. Deletes `overview_empty` and `render_detail_empty` entirely.

- **Pro**: no special cases; empty slot looks exactly like a genuine zero waveform
- **Con**: requires plumbing a pre-built buffer and placeholder state through the render calls; more structural change

### Option B — Grey box

Replace the waveform chrome with a plain filled rectangle. `overview_empty` and `render_detail_empty` each render a solid block in a dim colour, with no braille, no bar markers, no playhead indicator. Each deck slot uses a slightly different shade (Deck A / Deck B) to remain distinguishable.

- **Pro**: minimal code; completely honest — makes no claim about waveform state
- **Con**: loses the playhead position indicator for the empty slot

## Recommendation

Option B. It eliminates all pretence with the least code, and the absence of a waveform is itself informative ("no track loaded"). The current half-hearted rendering is worse than either extreme.
