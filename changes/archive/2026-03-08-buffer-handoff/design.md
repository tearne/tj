# Design: Smooth Buffer Handoff in Buffer Mode
**Status: Ready for Review**

## Approach

Replace the variable `chunk_size = window.len() / buf_cols` with a fixed `col_samp` (derived from zoom level only). Align the buffer anchor to a multiple of `col_samp` so that any two buffers at the same zoom level share the same column grid — column `c` always covers samples `[anchor + (c - buf_cols/2) * col_samp, anchor + (c - buf_cols/2 + 1) * col_samp)`.

With a shared column grid, overlapping columns between old and new buffers contain identical peak values and identical braille bytes. The viewport slides from old buffer into new buffer without any pixel-level difference — the handoff is invisible.

### Changes to the background thread

Replace the window-slice approach with per-column direct indexing:

```rust
// Align anchor to column grid.
let anchor = (pos_samp / col_samp) * col_samp;

let peaks: Vec<(f32, f32)> = (0..buf_cols).map(|c| {
    let offset    = c as i64 - (buf_cols / 2) as i64;
    let samp_start = (anchor as i64 + offset * col_samp as i64).max(0) as usize;
    let samp_end   = (samp_start + col_samp).min(mono.len());
    if samp_start >= mono.len() {
        return (0.0, 0.0);
    }
    let chunk = &mono[samp_start..samp_end];
    let mn = chunk.iter().cloned().fold(f32::INFINITY,     f32::min);
    let mx = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    (mn.max(-1.0), mx.min(1.0))
}).collect();

// Store with fixed col_samp — not derived chunk_size.
BrailleBuffer { grid, buf_cols, anchor_sample: anchor, samples_per_col: col_samp }
```

`half_buf`, `start`, `end`, `window`, and `chunk_size` are removed. The `actual_anchor` derivation is replaced by the aligned `anchor`.

## Tasks

1. ✓ Impl: Replace window-slice peak computation with fixed-grid per-column indexing; align anchor to `col_samp` boundary; store `col_samp` as `samples_per_col`.
   ✓ Impl: Early recompute trigger (`drift >= cols * 3/4`); fallback to last valid viewport on brief out-of-range frames to eliminate blackouts.
2. ✓ Verify: `cargo build`; manual test — buffer handoff invisible, no blackout frames.
3. ✓ Process: Archive `buffer-handoff`; update SPEC.md.
