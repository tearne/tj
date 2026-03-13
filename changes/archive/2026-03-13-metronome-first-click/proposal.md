# Proposal: Metronome First Click
**Status: Draft**

## Intent
When the metronome is activated, a click sounds immediately on key press. The first click should be suppressed so the metronome only begins clicking from the next scheduled beat, not at the moment of activation.

## Specification Deltas

### MODIFIED
- The metronome shall not emit a click on the beat that coincides with its activation; clicks begin from the following beat.
