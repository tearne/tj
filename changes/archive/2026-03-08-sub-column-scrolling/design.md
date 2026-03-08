# Design: Sub-Column Scrolling
**Status: Approved**

## Approach
Each braille character encodes a 2×4 dot grid. The current viewport snaps to full-character columns. By tracking position at dot-column resolution (half a character), we can produce a shifted row when at a half-character offset: for each character position `c`, combine the right dot-column of `buf[c]` with the left dot-column of `buf[c+1]` using a bit-shift operation.

Braille bit layout: left column = bits 0,1,2,6 (dots 1,2,3,7); right column = bits 3,4,5,7 (dots 4,5,6,8).

Shift function — takes right col of `a` → left col of result, left col of `b` → right col of result:
```
left  = ((a >> 3) & 0x07) | ((a >> 1) & 0x40)
right = ((b & 0x07) << 3) | ((b & 0x40) << 1)
result = left | right
```

The viewport calculation changes from full-column rounding to half-column rounding, with a `sub_col: bool` flag indicating whether to apply the shift. The adaptive frame period is also halved to target one dot-column advance per frame. The buffer already has enough width to supply the extra column needed when sub_col is true.

Beat marker column positions and viewport_centre_secs are adjusted by +0.5 columns when sub_col is true, keeping ticks visually aligned with the waveform.

## Tasks
1. ✓ Impl: Add `shift_braille_half(a: u8, b: u8) -> u8` helper
2. ✓ Impl: Update viewport calculation to track at half-column resolution and derive `sub_col`
3. ✓ Impl: Apply shift in the detail rendering loop when `sub_col` is true
4. ✓ Impl: Halve the adaptive poll duration to target one dot-column per frame
5. ✓ Impl: Adjust `viewport_centre_secs` by half a column when `sub_col` is true
6. Verify: Scrolling is visibly smoother; beat ticks stay aligned with waveform
7. Process: confirm ready to archive
