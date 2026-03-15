# Design: Inactive Deck Spectrum Analyser
**Status: Draft**

## Approach

The inactive deck servicing block (around line 336) handles smooth position advance, BPM auto-reject, and end-of-track pause. Spectrum computation is missing from this block.

Add spectrum computation for the inactive deck here, using the same logic as the active deck path:
- `half_period` derived from the inactive deck's `beat_period` (or 500ms fallback if analysing)
- `bar_period` = `beat_period * 8`
- Check `last_update` / `last_bg_update` elapsed times
- Call `compute_spectrum(&d.audio.mono, display_pos_samp_inactive, d.audio.sample_rate, d.filter_offset)`
- Update `d.spectrum.chars`, `d.spectrum.bg_accum`, `d.spectrum.bg`, `d.spectrum.last_update`, `d.spectrum.last_bg_update`

The inactive deck's `display_pos_samp` must be computed locally within the servicing block (it is currently computed later, in `inactive_render`). A local `pos_samp = seek_handle.position / channels` suffices — the same smooth-position logic is not needed here since the spectrum reads from `mono` at a sample index, and a one-frame-old position is imperceptible.

## Tasks

1. ✓ **Impl**: Add spectrum computation to the inactive deck servicing block

2. **Verify**: Build clean; confirm inactive deck spectrum animates while inactive
3. **Process**: Confirm ready to archive
