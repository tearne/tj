# Proposal: Overview Spectral Colour
**Status: Approved**

## Intent
The Overview waveform is currently a single colour (green). Colouring it by spectral content — blending between a bass colour and a treble colour based on the relative energy in each frequency band — gives the user an at-a-glance indication of the tonal character of each section of the track.

## Specification Deltas

### MODIFIED
- **Overview waveform colour**: Each column of the Overview waveform is coloured by the ratio of bass to treble energy in that column's audio window, blending continuously from orange (bass-heavy) through to cyan (treble-heavy). The split frequency is ~250 Hz. Colour is computed from pre-processed per-column energy data stored alongside the waveform peaks at load time, so there is no per-frame cost. The Detail view is unaffected and retains its existing colour.
