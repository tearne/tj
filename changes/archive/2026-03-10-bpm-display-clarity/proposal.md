# Proposal: BPM Display Clarity
**Status: Approved**

## Intent
The player has two layered BPM concepts that are currently conflated in the display:
1. **Detected BPM** (`base_bpm`) — the native tempo of the track as determined by analysis (or corrected via `h`/`H`). This governs beat grid positions.
2. **Playback tempo** (`bpm`) — the effective playback speed, which may differ from detected BPM when the user has applied a fine adjustment via `f`/`v`.

Currently the info bar shows a single BPM value that blends both concepts. After a `f`/`v` adjustment the displayed value no longer represents the track's native BPM, which is confusing.

## Specification Deltas

### MODIFIED
- The info bar distinguishes detected BPM from playback tempo when they differ. When no tempo adjustment is active (`bpm == base_bpm`), a single value is shown as before. When an adjustment is active, both values are shown, e.g. `120.0 bpm (→124.3)` or similar, making it clear that the track's native tempo is 120 and the current playback speed corresponds to 124.3 BPM.
- The exact format is an implementation concern; the requirement is that both values are legible and the relationship is unambiguous.
