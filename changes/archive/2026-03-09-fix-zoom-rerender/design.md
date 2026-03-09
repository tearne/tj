# Design: Fix Waveform Rerender at Maximum Zoom
**Status: Draft**

## Approach

Change the elapsed cap from `col_secs * 0.75` to `col_secs * 4.0`. This keeps the cap zoom-relative (preventing multi-column jumps from extreme OS delays) while ensuring it is always larger than the minimum poll duration of 8ms at every zoom level:

- 1s zoom, dc=100: cap = 4 × 10ms = 40ms  (was 7.5ms — less than poll_dur of 8ms ✗)
- 2s zoom, dc=100: cap = 4 × 20ms = 80ms  (was 15ms ✓, now more headroom)
- 32s zoom, dc=100: cap = 4 × 320ms = 1280ms (OS delays of >1s are not a concern)

4 columns per frame is a safe upper bound — it prevents any single delayed wakeup from causing a visible multi-column jump while never causing systematic drift.

## Tasks
1. ✓ Impl: Change `elapsed` cap from `col_secs * 0.75` to `col_secs * 4.0` in `src/main.rs`
2. ✓ Impl: Update the inline comment to explain the constraint
3. ✓ Verify: at 1s zoom, waveform scrolls stably for >30 seconds without any abrupt jump
4. ✓ Process: confirm ready to archive
