# Design: Wrap Tick Offset at Beat Period
**Status: Approved**

## Approach

Two call sites need the wrap applied:

1. **On adjustment** (`OffsetIncrease` / `OffsetDecrease` action handlers): after `offset_ms += 10` / `offset_ms -= 10`, compute `beat_period_ms = (60_000.0 / base_bpm as f64 / 10.0).round() as i64 * 10` (rounded to nearest 10ms to keep offset on the 10ms grid) and apply `offset_ms = offset_ms.rem_euclid(beat_period_ms)`. `base_bpm` is in scope at that point.

2. **On cache load** (background thread, `bpm_tx.send` block): after the existing 10ms snap, apply the same wrap using `entry.bpm` as the BPM source.

No other sites set `offset_ms` from user input in a way that could produce out-of-range values (tap-detected offsets are already derived via `rem_euclid` in `compute_tap_bpm`; re-detection offsets are also bounded by that function).

## Tasks

1. ✓ Impl: wrap `offset_ms` after each `OffsetIncrease` / `OffsetDecrease` action
2. ✓ Impl: wrap `offset_ms` on cache load
3. ✓ Process: archive
