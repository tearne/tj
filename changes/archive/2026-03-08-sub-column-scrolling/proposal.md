# Proposal: Sub-Column Scrolling
**Status: Approved**

## Intent
The detail waveform scrolls in discrete whole-character steps. Each braille character is 2 dot-columns wide, so the minimum scroll increment is one character — at wide zoom levels or low terminal resolution this produces a visible stutter. Scrolling at dot-column resolution (half a character) halves the step size and doubles step frequency at every zoom level, making motion smoother without changing the buffer or zoom system.

## Specification Deltas

### MODIFIED
- **Waveform Visualisation**: The detail waveform scrolls at dot-column resolution (half a braille character width) rather than full-character resolution. The viewport position is tracked at this finer granularity; when at a half-character offset, adjacent braille bytes are combined by bit-shifting to produce the shifted character row.

## Note
Previously set aside in favour of the adaptive-framerate approach, which improved timing regularity. Now being implemented on top of that to further reduce step size.
