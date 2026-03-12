# Design: Fix Filter Click
**Status: Draft**

## Approach

In `FilterSource`, the biquad state (`x1, x2, y1, y2`) is currently zeroed when `recompute_coeffs` is called (they are reset as part of struct initialisation and never explicitly preserved). The fix is simply to not reset them — keep the existing state values and only update the coefficients.

Looking at the code: `recompute_coeffs` only sets `self.b0..self.a2`; the state fields are separate. The zeroing happens on construction (`last_offset: 0` triggers a recompute on the first sample, with state already zero from struct init). Subsequent coefficient updates via `recompute_coeffs` do not zero state — so the click may actually come from the offset transition detection resetting state elsewhere, or from a brief moment where the wrong coefficients are applied.

Need to verify the exact source of the click during implementation.

## Tasks

1. ✓ **Impl**: Removed explicit state zeroing in `recompute_coefficients` — the state is now preserved across coefficient updates.
2. ✓ **Verify**: Step through filter offsets rapidly while audio plays; confirm no audible click or pop on any step or on reset to flat.
3. **Process**: Confirm ready to archive.
