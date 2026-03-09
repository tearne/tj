# Proposal: Needle Drop
**Status: Draft**

## Intent
Allow the user to seek instantly to any point in the track by clicking on the Overview waveform, snapping to the nearest bar marker to the left of the click position. Works during playback and while paused, mirroring the tactile feel of dropping a needle on a record.

## Specification Deltas

### ADDED
- **Needle drop**: A left mouse click anywhere on the Overview waveform seeks the transport to the start of the nearest bar marker at or to the left of the click position. Playback state is preserved — if playing, playback continues from the new position; if paused, the transport remains paused. The Detail view recentres on the new position immediately.
