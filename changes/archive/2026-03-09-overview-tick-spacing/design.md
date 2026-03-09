# Design: Overview Tick Spacing and Waveform View Terminology
**Status: Draft**

## Approach

### Adaptive tick spacing
`bar_tick_cols` currently hard-codes a 4-bar interval. Extend it to:
1. Start at `bars = 4` (minimum).
2. Compute tick columns for the full track.
3. Find the minimum gap between any two adjacent tick columns.
4. If that gap < 2 columns (no blank character between them), double `bars` and repeat from step 2.
5. Return `(Vec<usize>, u32)` — the tick columns and the current bars-per-tick value.

### Legend
The bars-per-tick value is formatted as e.g. `"4 bars"` and overlaid on the top-right of the Overview. During row rendering, switch to `enumerate()` on the row index; in row 0, replace the rightmost `legend.len()` columns with the legend characters, coloured `DarkGray`.

### Waveform view names
"Overview" and "Detail view" are already the terms used in the spec and informally in the code. No code changes required — terminology is a spec-only concern addressed in the proposal.

### SPEC.md
Remove the `### Detail Waveform Render Modes` reference (already done). Update the overview tick description. The view names "Overview" and "Detail view" are already consistent in the spec after the terminology change.

## Tasks
1. ✓ Impl: Extend `bar_tick_cols` to return `(Vec<usize>, u32)` with adaptive doubling
2. ✓ Impl: Update Overview rendering — use `enumerate()`, inject legend in top-right of row 0
3. ✓ Impl: Update SPEC.md — overview tick spacing and legend behaviour
4. ✓ Verify: legend shows correct bar count; ticks never overlap at any terminal width or BPM
5. ✓ Process: confirm ready to archive
