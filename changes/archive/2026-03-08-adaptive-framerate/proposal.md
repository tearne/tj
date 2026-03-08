# Proposal: Adaptive Framerate
**Status: Ready for Review**

## Intent
The detail waveform scrolls in discrete column steps. With a fixed frame rate, column boundaries fall at irregular intervals relative to frame boundaries — sometimes a step occurs after one frame, sometimes after two — and this irregular timing is visually distracting. Setting the frame period equal to the column duration ensures every frame advances the viewport by exactly one column, making steps perfectly regular.

The manual FPS control added previously is removed in favour of this approach.

## Specification Deltas

### MODIFIED
- **Rendering**: The render frame period adapts to the current zoom level and detail panel width, targeting one column advance per frame. At very tight zoom (high frame rate requirement) it is capped at ~120 fps; at very wide zoom it is capped at ~5 fps to remain responsive to input. The manual `f`/`F` FPS control and the current fps display in the key hints are removed.

### REMOVED
- The user-adjustable render framerate (`f`/`F` keys, discrete fps levels, fps display in key hints).
