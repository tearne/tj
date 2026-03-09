# Proposal: Detail View Tick Visibility
**Status: Draft**

## Intent
Beat tick marks in the Detail view are rendered at half-column resolution (⡇ or ⢸) in `DarkGray`, making them easy to miss at fast scroll speeds. Increasing their brightness improves visibility without breaking the half-column synchronisation with the waveform.

Note: using full-character-width marks (⣿) was investigated but rejected — isolated ⣿ bytes lose half-column precision, causing the tick to oscillate ±0.5 col relative to the waveform. The half-column characters (⡇/⢸) are the correct output of the same pipeline that renders waveform content and must be preserved.

## Specification Deltas

### MODIFIED
- **Detail view beat markers**: Beat tick marks are rendered in a brighter colour than the surrounding background, ensuring they remain visible at all scroll speeds.
