# Design: Smooth Scrolling Detail Waveform
**Status: Ready for Review**

## Approach

The background thread pre-renders a braille buffer **3× the visible width**
(`buf_cols = 3 * screen_cols`). The UI thread computes a `viewport_start`
column offset into this stable buffer each frame based on the current playback
position — no recomputation needed for normal scrolling.

The buffer is only recomputed when:
- zoom or window dimensions change, or
- the playhead has drifted more than `screen_cols` columns from the buffer
  anchor (i.e. the viewport would exceed the buffer) — roughly once per
  screen-width of playback (~12 s at 4 s zoom, 200 cols).

On seek the drift is large → immediate recompute. A blank frame may appear
while the thread catches up; this is acceptable.

### BrailleBuffer struct

Replaces the bare `Vec<Vec<u8>>` stored in the shared Arc:

```rust
struct BrailleBuffer {
    grid:            Vec<Vec<u8>>, // rows × buf_cols braille bytes
    buf_cols:        usize,        // total buffer width (= 3 × screen_cols)
    anchor_sample:   usize,        // mono-sample index at the buffer centre
    samples_per_col: usize,        // mono samples per braille column
}
```

An empty sentinel (`buf_cols = 0`) is used as the initial value before the
first compute.

### Background thread (changes from current)

Invalidation condition changes from `center_col != last_center_col` to:

```
drift_cols = |pos_samp − anchor_sample| / samples_per_col
recompute  = cols != last_cols
          || rows != last_rows
          || zoom != last_zoom
          || drift_cols >= cols          // viewport within 1 screen of buffer edge
```

On recompute: render `buf_cols = 3 × cols` columns centred on `pos_samp`,
store a new `BrailleBuffer` with `anchor_sample = pos_samp`.

### UI thread (changes from current)

Each frame, after reading the Arc:

```
delta_cols    = (pos_samp − anchor_sample) / samples_per_col   (signed)
viewport_start = buf_cols/2 + delta_cols − screen_cols/2
```

- If `viewport_start` is in range `[0, buf_cols − screen_cols]`: display
  `grid[row][viewport_start .. viewport_start + screen_cols]` for each row.
- Otherwise (seek in progress / first frame): display blank rows.

Beat tick columns are still computed with `beat_line_cols(…, pos_secs,
zoom_secs, screen_cols)` — unchanged, because the visible window is still
centred on `pos_secs`.

### Centre line

The centre column is always `screen_cols / 2` — unchanged.

## Tasks

1. ✓ Impl: Add `BrailleBuffer` struct; update background thread to pre-render
   `3×cols` buffer, recompute only on zoom/resize/edge-drift instead of every
   center-col advance.
2. ✓ Impl: Update UI thread — compute `viewport_start`, display the viewport
   slice of the buffer grid; blank rows if out of range.
3. ✓ Verify: `cargo build`; manual test — smooth scrolling, no stutter, beat
   ticks stable relative to waveform.
4. ✓ Process: Archive `smooth-scrolling` and `braille-rendering`; update SPEC.md.
