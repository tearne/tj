# Proposal: Display Position Invariants
**Status: Approved**

*(Retrospective — implemented directly from note without prior proposal/design approval.)*

## Intent

Two implicit contracts in the rendering and offset-handling code were revealed by bugs
during the `key-direction-audit` experiment. Neither was expressed in code structure,
making them easy to violate when either area is next touched. This change makes both
invariants structural rather than documentary.

## Specification Deltas

### ADDED

- `marker_view_start` (the view-start used for tick and cue screen-column calculation)
  is derived from the exact `display_pos_samp`, not the half-column-quantized buffer
  anchor. The two values are named and computed separately in `render_detail_waveform`.

- Offset key steps are applied through a single `apply_offset_step(d, delta_ms)` helper.
  The display-position delta is always the raw ±10 ms step; it is never derived from
  `new_offset − old_offset`, which diverges from the step when `rem_euclid` wraps.
