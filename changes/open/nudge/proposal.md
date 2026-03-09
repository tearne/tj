# Proposal: Nudge
**Status: Draft**

## Intent
Nudge temporarily adjusts playback speed by ±10%, allowing the user to drift the track forward or backward in time relative to an external reference (e.g. another deck). Works during both playback and pause (while paused, nudge has no audible effect but shifts the transport position at ±10% of normal speed while held).

## Specification Deltas

### ADDED
- **Nudge**: Holding a nudge key applies a continuous ±10% speed offset to playback. Releasing the key returns to normal speed immediately. The nudge state is indicated in the UI. Nudge works during playback and while paused.
