# Design: Configurable Detail Waveform Height
**Status: Ready for Review**

## Approach

Add a `detail_height: usize` variable (default 8) to `tui_loop`. The layout changes `Constraint::Min(6)` for the detail panel to a two-part split: `Constraint::Length(detail_height)` for the waveform plus `Constraint::Min(0)` for blank space below.

`{` decreases and `}` increases `detail_height`, clamped to `[1, available_inner_height - fixed_rows]` where `fixed_rows = 5` (BPM line + overview + beat indicator + status + key hints).

Key hints line updated to show `{`/`}` controls.

No new atomics needed — `detail_rows` is already written from `chunks[2].height` each frame, so the background thread automatically adapts when height changes.

## Tasks

1. ✓ Impl: Add `detail_height` variable; split detail area into `Length(detail_height)` + `Min(0)`; add `{`/`}` key handlers with clamping; update key hints.
2. Verify: `cargo build`; manual test — height adjusts correctly, waveform renders at new size, no stutter at small heights.
3. Process: Archive `detail-height`; archive `smooth-scrolling` and `braille-rendering`; update SPEC.md for all three.
