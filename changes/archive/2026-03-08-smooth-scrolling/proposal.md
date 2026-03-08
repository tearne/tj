# Proposal: Smooth Scrolling Detail Waveform
**Status: Ready for Review**

## Intent

The detail waveform currently recomputes its entire braille grid every time the
playhead advances by one column (~20ms at 4s zoom, 200 cols). The UI receives
new grid content ~50 times per second, causing ratatui to re-render the full
waveform every frame — visible as periodic stutter. Beat tick rendering also
oscillates because the grid shifts constantly, causing tick-column bytes to
alternate between zero and non-zero.

Pre-rendering a buffer wider than the visible area eliminates both problems: the
buffer is stable while the viewport pans through it, so most frames require no
computation and the displayed content changes by exactly one column per
column-advance — true smooth scrolling.

## Specification Deltas

### MODIFIED

**Waveform Visualisation:**
- The detail waveform scrolls smoothly as playback advances. The displayed
  content shifts by one column at a time rather than in periodic full-grid
  jumps.
- On zoom change or window resize the display updates immediately (within one
  background thread cycle, as before).
- Seeking causes an immediate recompute of the buffer; a brief blank or stale
  frame may be shown while the background thread catches up.
