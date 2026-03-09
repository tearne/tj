# Design: Start Rendering Fixes
**Status: Approved**

## Approach

### Fix 1: Pre-track blank
In the background thread peak computation, columns whose raw sample position is negative were clamped to `samp_start=0`, causing them to show the first audio chunk. Fix: check `raw_start < 0` and return an invalid peak range `(1.0, -1.0)`, which render_braille skips (renders as blank — no dots).

### Fix 2: Tick oscillation with sub_col
Sub-column scrolling shifts the viewport by half a character when `sub_col` flips. At wide zoom levels, `poll_dur ≈ half_col_secs` so sub_col flips every frame. Tick marks encoded as isolated `0xFF` bytes in the pre-rendered buffer produce completely different braille characters on alternating frames when processed through `shift_braille_half` — solid ⣿ one frame, split ⡇+⢸ the next — causing visible oscillation.

The fix is to compute tick marks directly in **display space** rather than encoding them in the buffer:
- For each beat, compute its position relative to the quantised viewport centre: `disp_half = round((t_samp - view_start) / half_spc)`.
- Place ⡇ (0x47, left-column dots) for even `disp_half`, ⢸ (0xB8, right-column dots) for odd.
- No `shift_braille_half` processing is needed; ticks are already at half-column resolution.

This makes tick positions lock-step with the waveform and immune to sub_col flipping.

### Fix 3: Tick–waveform desync after the display-space fix
After fix 2, occasional single-column snaps remained. The root cause: `view_start` for ticks was derived from `smooth_display_samp` (a raw f64), while the waveform viewport uses a quantised reference: `anchor + delta_half × half_col_samp`. The two differ by up to `half_col_samp / 2`, causing ticks to round to a different half-column than the waveform.

The fix is to hoist `delta_half` and `half_col_samp` out of the viewport block and use them to compute the tick's `view_start`:
```
visual_centre = anchor + delta_half × half_col_samp
view_start    = visual_centre − centre_col × col_samp
```
Both waveform and ticks now derive from the identical quantised reference.

### Fix 4: Seek snap to column boundary
After a seek, `smooth_display_samp` is snapped to the nearest **full column** boundary (not just to `pos_samp`). This ensures `sub_col = false` after every seek, preventing a permanent half-column offset when the seek distance happens to land on an odd number of half-columns.

## Tasks
1. ✓ Impl: Return `(1.0, -1.0)` for buffer columns whose raw sample position is negative (renders as blank)
2. ✓ Impl: Enable sub_col at all zoom levels (remove the ~30Hz threshold)
2b. ✓ Impl: Display-space tick rendering — compute ticks from quantised viewport centre, draw ⡇/⢸ at half-column precision without shift_braille_half
2c. ✓ Impl: Hoist delta_half — derive tick view_start from `anchor + delta_half × half_col_samp` to match waveform exactly
2d. ✓ Impl: Snap smooth_display_samp to nearest column boundary after large drift
3. ✓ Verify: Left side shows blank at start; ticks present; waveform and ticks scroll smoothly in lock-step at all zoom levels
4. Process: confirm ready to archive
