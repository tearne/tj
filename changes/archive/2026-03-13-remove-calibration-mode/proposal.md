# Proposal: Remove Calibration Mode
**Status: Approved**

## Intent

The `~` latency calibration mode (synthetic click + travelling pulse marker) is superseded by the `[`/`]` live latency adjustment, which provides a more intuitive workflow: tap to lock BPM against heard beats, then nudge latency until ticks align with waveform peaks. The click-based mode is no longer needed.

## Specification Deltas

### REMOVED

- Calibration mode (`~` toggle) is removed entirely.
- The synthetic click tone fired at 60 BPM during calibration is removed.
- The travelling calibration pulse marker (cyan, double-width tick) is removed.
- The playhead flash (bright red) on pulse coincidence is removed.
- The calibration-mode info bar (`lat:Nms  d/c adjust  ~ exit`) is removed.
- The zoom reset on calibration entry/exit is removed.
- The restriction on entering calibration mode while playing is removed (no longer relevant).
- `~` / `calibration_toggle` key binding is removed.

### MODIFIED

- `d`/`c` revert to nudge-only behaviour at all times (no latency adjustment path).
- `audio_latency_ms`, `[`/`]` live adjustment, and `lat:Xms` info bar display are retained.
- The spectrum analyser hidden-in-calibration-mode restriction is removed.
