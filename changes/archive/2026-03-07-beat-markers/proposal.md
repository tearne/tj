# Proposal: Beat Markers on Waveform
**Status: Ready for Review**

## Intent
Overlay beat/bar position markers on the waveform views so the user can visually verify beat alignment and the effect of phase offset adjustments.

## Specification Deltas

### ADDED

**Beat markers**:
- The overview waveform displays a tick at the bottom of the canvas at each bar position (every 4 beats).
- The detail waveform displays a faint vertical line at each beat position within the visible window.
- Both sets of markers are derived from the detected BPM and current phase offset, so they shift immediately when the offset is adjusted.
