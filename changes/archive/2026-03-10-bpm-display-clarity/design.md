# Design: BPM Display Clarity
**Status: Draft**

## Approach

In the info bar, replace the single BPM span with conditional display:

- When `bpm == base_bpm` (no adjustment): show `120` — no decimal, no "bpm" suffix. The amber beat-flash applies to this span as now.
- When they differ: show two spans — `120 ` as plain text (no flash), then `(124.4)` with the amber beat-flash applied to the bracketed value instead. No "bpm" suffix in either case.

The comparison uses a small epsilon (`(bpm - base_bpm).abs() < 0.05`) to avoid floating-point noise.

The adjusted value is shown to one decimal place; the base value is shown as an integer.

## Tasks

1. **Impl**: Update the info bar BPM rendering: single integer span with flash when unadjusted; two spans (plain base + flashing bracketed adjusted) when adjusted; drop "bpm" suffix in both cases
2. **Verify**: Confirm integer display at rest with beat flash; confirm `(124.4)` appears and pulses after `f`/`v`; confirm base value stops flashing when adjusted; confirm single value resumes after `h`/`H`
3. **Process**: Confirm ready to archive
