# Design: BPM Fine Adjustment
**Status: Approved**

## Approach

Mirror the existing `h`/`H` (halve/double) pattern: adjust `bpm` by ±0.1, clamp to 40.0–240.0, persist to cache immediately. Add `f`/`F` to the help popup. BPM is stored as `f32` so 0.1 increments are exact enough for display purposes (rounded to one decimal in the info bar, or left as-is since it's already shown as integer — need to decide at implementation time).

Actually, BPM is currently displayed as `bpm as u32` in the info bar. Fine adjustments of 0.1 won't be visible at integer resolution. Update display to one decimal place (e.g. `120.3 bpm`).

## Tasks

1. **Impl**: Add `f`/`v` key handlers — adjust `bpm` by ±0.1, clamp to 40.0–240.0, persist to cache
2. **Impl**: Update info bar BPM display from integer to one decimal place; update help popup
3. **Verify**: Confirm F/V adjust BPM by 0.1; confirm display updates; confirm cache persisted
4. **Process**: Confirm ready to archive
