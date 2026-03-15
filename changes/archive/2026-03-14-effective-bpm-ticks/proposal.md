# Proposal: Effective BPM Tick Marks
**Status: Approved**

## Intent

Beat tick marks in the detail waveform should reflect the effective playback BPM, not the detected BPM. When two decks are set to the same effective BPM, their tick grids should be visually identical and appear locked together. Ticks must also remain anchored to the waveform content — they must not float when the effective BPM is adjusted.

## Specification Deltas

### MODIFIED

- **Detail waveform column density**: the number of audio samples shown per screen column is scaled by `bpm / base_bpm` per deck. This means the waveform viewport is expressed in playback-time columns rather than raw audio-sample columns. At `bpm == base_bpm` (no adjustment), behaviour is unchanged.
- **Beat tick marks**: because the column grid is now scaled by playback speed, ticks placed at `base_bpm` sample spacing naturally appear at `bpm`-spaced columns — they stay anchored to the waveform content and simultaneously match the effective BPM grid across decks.
- **Beat flash**: unchanged in mechanism; continues to fire at the effective BPM rate.

## Problem with naive substitution

Replacing `base_bpm` with `bpm` in the tick period formula causes ticks to float relative to the waveform. The correct fix is to scale the column grid itself, so that both the waveform and the ticks — still computed in sample space — appear at the effective BPM density.
