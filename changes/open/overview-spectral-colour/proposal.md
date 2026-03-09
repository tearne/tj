# Proposal: Overview Spectral Colour
**Status: Note**

## Intent
The Overview waveform is currently a single colour (green). Colouring it by spectral content — blending between a bass colour and a treble colour based on the relative energy in each frequency band — would give the user an at-a-glance indication of the tonal character of each section of the track.

## Unresolved
- What frequency split defines "bass" vs "treble"? A simple low/high split (e.g. below/above ~250 Hz) is easy to compute; a multi-band approach would be richer but more complex.
- What colours work well? Options: bass = red/orange, treble = blue/cyan, with a blend in between. Needs to remain legible on dark terminals.
- How is the colour computed per column? Options:
  - Energy ratio: `bass_energy / (bass_energy + treble_energy)` → maps to a colour gradient.
  - Dominant band: whichever band has more energy determines the colour, with intensity from amplitude.
- Spectral analysis per column adds cost to the overview computation — needs to remain negligible (overview is rendered fresh each frame).
- Should this apply to the Detail view as well, or Overview only?
