# Design: Effective BPM Tick Marks
**Status: Draft**

## Approach

Scale the braille buffer's `samples_per_col` per deck by `bpm / base_bpm`. With this scaling:

- Tick marks computed in sample space at `base_bpm` spacing appear at `60 / (bpm × col_secs)` columns — identical across decks at the same effective BPM. ✓
- The waveform viewport advances at `1 / col_secs` columns per real second for both decks regardless of their individual `bpm / base_bpm` ratios. ✓
- Ticks stay anchored to waveform content because both are in the same (scaled) column grid. ✓

### SharedDetailRenderer

Add two atomics storing per-deck speed ratio as fixed-point (ratio × 65536):

```rust
speed_ratio_a: Arc<AtomicUsize>,  // (bpm / base_bpm) × 65536
speed_ratio_b: Arc<AtomicUsize>,
```

The background thread computes per-deck `col_samp`:

```rust
let col_samp_a = (col_secs * sample_rate_a as f64 * speed_ratio_a as f64 / 65536.0) as usize;
let col_samp_b = (col_secs * sample_rate_b as f64 * speed_ratio_b as f64 / 65536.0) as usize;
```

Each buffer is computed with its own `col_samp`; the two buffers are no longer required to share identical column grids.

### Drift-snap

The inactive and active deck drift-snap formulas currently use `col_secs * sample_rate`. Update to `col_secs * sample_rate * bpm / base_bpm` to stay consistent with the scaled grid.

### Beat flash and tick marks

Revert the Task 1 change — restore `base_bpm` in the tick spacing and beat flash formulas. With the scaled column grid, they naturally produce the correct visual result.

### BPM ratio updates

Store the ratio on every `BpmIncrease` / `BpmDecrease` / `BaseBpmIncrease` / `BaseBpmDecrease` / `TempoReset` action, and on deck load (ratio = 1.0 initially).

## Tasks

1. ✓ **Revert**: Restore `base_bpm` in tick mark and beat flash formulas (undo Task 1)
2. ✓ **Impl**: Add `speed_ratio_a` / `speed_ratio_b` atomics to `SharedDetailRenderer`; background thread uses per-deck `col_samp`
3. ✓ **Impl**: Store speed ratio to renderer on deck load and on every BPM-changing action; update drift-snap formulas
3b. ✓ **Impl**: Beat jump uses `base_bpm` (not `bpm`) so jumps land on tick marks at all speed ratios
4. **Verify**: Build clean; both decks at same effective BPM show identical, waveform-anchored tick grids; adjusting BPM moves ticks with the waveform; beat jumps land on ticks
5. **Process**: Confirm ready to archive
