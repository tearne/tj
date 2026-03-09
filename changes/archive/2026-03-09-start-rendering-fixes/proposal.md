# Proposal: Start Rendering Fixes
**Status: Ready for Review**

## Intent
Two visual defects appear at the start of playback while the waveform is scrolling in from the right:

1. The left portion of the detail view (representing time before the track starts) shows audio content from position 0 rather than silence, producing a solid or noisy waveform that varies with zoom level.
2. Beat tick marks in the detail view flicker until the playhead has advanced past the left edge of the screen.

During implementation, two further defects were diagnosed and fixed:

3. At wide zoom levels, beat tick marks oscillate relative to the waveform when seeking backward. Root cause: tick marks encoded as isolated bytes in the pre-rendered buffer produce different braille characters on alternating sub-column frames when processed through the half-column shift function.
4. Even after switching to display-space tick rendering, ticks occasionally desync from the waveform by a snap. Root cause: the tick position was derived from the raw smooth display position rather than the quantised half-column reference used by the waveform viewport.

## Specification Deltas

### ADDED
- **Rendering — Tick marks in display space**: Beat tick marks must be computed directly in display space from the quantised viewport centre, not encoded in the pre-rendered waveform buffer. Isolated marks in buffer space produce completely different braille characters when processed through the half-column shift, causing visible oscillation on every sub-column step. Computing marks in display space means no shift processing is needed and marks scroll smoothly with the waveform.
- **Rendering — Seek snap to column boundary**: When snapping the display position after a large drift (seek or startup), snap to the nearest full column boundary rather than to the raw sample position. This ensures `sub_col = false` after every seek, preventing the viewport from being permanently offset by half a column.

### MODIFIED
- **Waveform Visualisation**: Buffer columns representing sample positions before the start of the track render as silence (zero amplitude) rather than mirroring the content at position 0.
- **Rendering — Consistent tick and viewport centre**: Strengthened: tick positions must be derived from the same quantised half-column reference as the waveform viewport (`anchor + delta_half × half_col_samp`), not from the raw smooth display position. The two can differ by up to half a column, causing visible oscillation.
- **Rendering**: Beat tick marks do not flicker or oscillate at any zoom level or playback position.
