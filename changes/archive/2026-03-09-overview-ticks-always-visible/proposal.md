# Proposal: Overview Ticks Always Visible
**Status: Draft**

## Intent
Bar markers in the Overview are currently only drawn where the waveform cell is empty (byte == 0), so they are hidden wherever the waveform is dense. At high-energy sections of a track the markers can disappear entirely, making it hard to judge position at a glance.

## Specification Deltas

### MODIFIED
- **Overview bar markers**: Bar markers are drawn at the top and bottom rows of the Overview only, always visible regardless of waveform content. The waveform is unobstructed in the middle rows.
