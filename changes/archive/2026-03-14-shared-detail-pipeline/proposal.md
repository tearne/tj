# Proposal: Shared Detail Waveform Pipeline
**Status: Approved**

## Intent

Both detail waveforms must remain visually stable and in sync regardless of which deck is active. Currently the inactive deck's waveform wobbles because frame timing is derived from the active deck, advancing the inactive deck's viewport at the wrong rate.

## Specification Deltas

### ADDED

- **Detail info bar**: a single shared row above both detail waveforms showing the common zoom level.

### MODIFIED

- **Zoom level**: previously per-deck (`-`/`=` on the active deck); now a single shared value applied to both detail waveforms simultaneously.
- **Detail height**: previously per-deck (`{`/`}` on the active deck); now a single shared value applied to both detail waveforms simultaneously.
- **Detail waveform render pipeline**: both waveforms are rendered by a single shared background thread at the same zoom and `samples_per_col`, ensuring their column grids are identical and both viewports advance at the same rate each frame.
- **Layout**: the detail section gains a shared info bar as its first row (above both waveforms).

### REMOVED

- **Per-deck zoom indicator** in each deck's info bar — superseded by the shared detail info bar.

## Problem

When two decks are playing and the user switches active decks, the inactive deck's detail waveform wobbles visually. Audio is unaffected.

The cause is that frame timing (`poll_dur`, `elapsed`) is derived from the **active deck's** zoom level and renderer. The inactive deck's smooth display position is therefore advanced at the wrong rate — and capped by the wrong `col_secs` bound — whenever the two decks are at different zoom levels. The inactive deck's background braille thread also computes on its own schedule, which may not align with the active deck's frame period, causing the UI to read a stale or misaligned buffer segment.

## Proposed change

### Shared zoom

Lock both detail waveforms to a single `zoom_idx`, stored as a global rather than per-deck. `{`/`}` adjust it for both simultaneously. The frame period and `elapsed` cap are then consistent for both decks, eliminating the rate mismatch.

### Shared render pipeline

Replace the two per-deck background braille threads with a single shared thread that renders both waveforms in the same pass. Both buffers are computed at the same zoom, from the same `samples_per_col`, ensuring their column grids are identical. The UI reads from both buffers within the same frame, eliminating the timing skew between them.

### Detail info bar

With zoom no longer belonging to either deck individually, a shared info bar is placed between the two detail waveforms (or below both) to surface it. Content (at minimum): the common zoom level. This bar replaces the per-deck zoom field in each deck's info bar.

## Decisions

- **Detail info bar position**: above both waveforms, as the first row of the detail section.
- **Detail info bar content**: zoom level only. No per-deck position or beat data.
- **Detail height**: unified — a single shared value, adjusted by `{`/`}` globally.
- **Smooth display position updates**: unified — both decks' positions advance from the same `elapsed` value each frame, derived from the shared zoom and renderer.
