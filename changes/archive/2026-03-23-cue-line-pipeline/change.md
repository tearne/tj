# Cue Line Pipeline
**Type**: Fix
**Status**: Done

## Problem

The cue line in the detail waveform is computed per-frame from the exact
`display_pos_samp`, while the waveform viewport uses a half-column-quantized
position derived from `buf.anchor_sample`. The different coordinate bases can
cause the cue line to drift relative to the waveform.

## Fix

Compute `cue_screen_col` from `buf.anchor_sample` and `buf.samples_per_col` in
buffer space, then map to screen via `viewport_start` — the same coordinate
transform the waveform uses. Remove the per-frame `marker_view_start`
computation entirely.

```
cue_buf_col = buf_cols/2 + (cue_sample - anchor_sample) div samples_per_col
cue_screen_col = cue_buf_col - viewport_start   (if in [0, detail_width))
```

## Log

Added `cue_buf_col: Option<usize>` to `BrailleBuffer`. Background thread computes
it from `cue_sample` (passed via `cue_sample_a/b: Arc<AtomicI64>`, -1 = None) using
the same anchor and `samples_per_col` as the waveform and ticks. Draw thread maps
it to screen via `viewport_start` — one line, identical to tick extraction. Previous
per-frame computation using `display_pos_samp` removed entirely. Spec updated to
describe waveform, ticks, and cue as all rendered in the same buffer pipeline.

Investigation into remaining ±0.5 column jitter: the cue is a colored character-cell
overlay and can only be positioned at integer columns, while the waveform scrolls at
half-column (braille dot) resolution. This mismatch is inherent to terminal rendering
and accepted as a known limitation. Approaches tried and rejected: sub_col -1 shift
(made it worse — alternated between two positions), baking 0xFF into the grid (same
visual result since coloring is still integer-column). Clean overlay retained.
