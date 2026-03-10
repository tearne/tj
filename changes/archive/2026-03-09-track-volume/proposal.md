# Proposal: Track Volume Control
**Status: Approved**

## Intent
Allow the user to adjust the playback volume of the current track at runtime, displayed in the UI.

## Specification Deltas

### ADDED
- **Volume control**: The playback volume is adjustable at runtime using `↑` (increase) and `↓` (decrease) in 5% steps, from 0% to 100%. The current volume level is displayed in the UI. Volume changes take effect immediately without interrupting playback. Volume is not persisted between sessions.
