# Deck 2 Cue Overlap Colour
**Type**: Fix
**Status**: Done

## Problem

In the detail waveform, when deck 2's playhead overlaps the cue line, the bottom
row does not render in the cue colour (magenta), unlike deck 1.

## Root Cause

`is_edge_row` is computed as `r == 0 || r + 1 == waveform_rows`. For deck A,
`waveform_rows = detail_panel_rows - 1` (tick row subtracted). For deck B,
`waveform_rows = detail_panel_rows`. The buffer is rendered at
`detail_panel_rows - 1` rows for both decks. So deck B's `waveform_rows` is one
more than the buffer height, making the actual last buffer row (`r = waveform_rows - 2`)
not detected as an edge row.

## Fix

Cap the edge row check at actual buffer height: `actual_rows = buf.grid.len().min(waveform_rows)`,
then `is_edge_row = r == 0 || r + 1 == actual_rows`.

## Log

One-line fix: cap edge row check at `buf.grid.len().min(waveform_rows)` so deck B's
actual last buffer row is correctly identified as the bottom edge.
