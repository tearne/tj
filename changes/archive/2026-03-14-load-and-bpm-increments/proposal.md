# Proposal: Load Behaviour and BPM Increment Refinements
**Status: Draft**

## Intent

Two small refinements to playback and BPM control:

1. Tracks should not begin playing automatically on load — the user decides when to start.
2. BPM adjustment steps are too coarse for fine-tuning; both adjustment keys should operate at 0.01 BPM resolution.

## Specification Deltas

### MODIFIED

- **Track load behaviour**: previously began playback immediately on load; now loads paused. The user starts playback with `Space+Z` as normal.
- **`f` / `v` (playback BPM)**: step size reduced from ±1.0 to ±0.01 BPM.
- **`F` / `V` (detected BPM)**: step size reduced from ±0.1 to ±0.01 BPM.
