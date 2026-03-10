# Proposal: BPM Fine Adjustment
**Status: Approved**

## Intent
Allow the user to nudge the detected BPM up or down in 0.1 BPM increments at runtime, for cases where the detected BPM is close but not exact. Complements the existing halve/double corrections.

## Specification Deltas

### ADDED
- `bpm_increase` — increases the current BPM by 0.1. Dev config key: `f`.
- `bpm_decrease` — decreases the current BPM by 0.1. Dev config key: `v`.
- Adjusted BPM is persisted to the cache immediately (same behaviour as halve/double).
- BPM is clamped to the existing range (40–240).
