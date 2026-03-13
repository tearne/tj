# Proposal: Wrap Tick Offset at Beat Period
**Status: Approved**

## Intent

`offset_ms` can currently grow unboundedly in either direction. Any value beyond one beat period is equivalent to a value within `[0, beat_period_ms)` — the tick positions are identical. Wrapping on each adjustment keeps the stored value canonical, makes the display meaningful at a glance, and removes the need for the user to reason about which direction to nudge when the offset has drifted far.

## Specification Deltas

### MODIFIED

- After each `offset_increase` or `offset_decrease` action, `offset_ms` shall be wrapped into `[0, beat_period_ms)` using modular arithmetic (`rem_euclid`).
- The wrap uses the current `base_bpm` to compute `beat_period_ms` at the moment of adjustment.

## Notes

- Wrapping on adjustment (not continuously per-frame) means a subsequent BPM change does not silently alter the phase — the stored offset remains stable until the user next adjusts it.
- The range `[0, beat_period_ms)` is preferred over a signed range because offset has no meaningful negative interpretation — it is a phase within a repeating cycle.
- The same wrap should be applied when offset is loaded from cache, in case it was written before this change or by an external edit.
