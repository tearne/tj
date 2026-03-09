# Proposal: Track Volume Control
**Status: Draft**

## Intent
Allow the user to adjust the playback volume of the current track at runtime, displayed in the UI.

## Specification Deltas

### ADDED
- **Volume control**: The playback volume is adjustable at runtime. The current volume level is displayed in the UI. Volume changes take effect immediately without interrupting playback.

## Unresolved
- Key bindings: likely `↑`/`↓` or dedicated keys — to be decided when keyboard mapping is designed.
- Volume range and step size (e.g. 0–100% in 5% steps).
- Whether volume is persisted between sessions (per-track or global).
