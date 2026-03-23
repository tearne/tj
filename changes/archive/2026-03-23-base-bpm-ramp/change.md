# Base BPM Ramp
**Type**: Proposal
**Status**: Done

## Log

Implemented time-based two-tier ramp on the four `BaseBpm*` actions. Key
discoveries during implementation:

- OS key-repeat fires as `Press` events (not `Repeat`) on the test terminal,
  so the ramp uses a gap-based reset (80 ms) rather than checking
  `KeyEventKind::Repeat`. This makes it robust regardless of terminal behaviour.
- A `Release`-based reset caused the step to snap back to 0.01 on every
  re-press; replaced with the gap check so a quick release-and-repress
  continues the current tier.

Final tiers: < 3 s → 0.01 BPM/event, ≥ 3 s → 0.05 BPM/event.

When holding a base BPM key (S/X, F/V), the rate of change accelerates the longer the key is held, allowing large BPM corrections quickly while still permitting fine single-step adjustments on a tap.
