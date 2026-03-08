# Proposal: FPS Control
**Status: Approved**

## Intent
Allow the user to adjust the render framerate at runtime, to tune the smoothness of the scrolling waveform against CPU cost.

## Specification Deltas

### ADDED
- The render framerate is user-adjustable at runtime with `f` (decrease) and `F` (increase), cycling through discrete levels: 10, 15, 20, 30, 60, 120 fps. The default is 30 fps.
- The current framerate is shown in the key hints line.
