# Proposal: Soften Beat Flash
**Status: Draft**

## Intent
The beat flash indicator is visually distracting — its sudden full-brightness appearance competes with the waveform and draws the eye away from the scroll. A subtler flash would preserve beat awareness while reducing visual noise.

## Unresolved
- What form should the softer flash take? Options include:
  - Dimmer colour (e.g. dark gray instead of bright white/yellow)
  - Narrower flash window (shorter on-time per beat)
  - Fade in/out rather than hard on/off (requires sub-beat timing)
  - A different indicator design altogether (e.g. a small marker rather than a full panel)

### Steer
- I propose we rework the overall UI a little.  I'd like a single information bar at the top which wraps if the terminal isn't wide enough.  We already have BPM, Offset, and jump unit.  We'd add `[h]elp` to indicate that the user can press `h` to see a little popup to explain the key mapping.  We also add the play indicator, and the detail zoom level indicator.  I'd like to make the bar more compact, for example using a little triangle or pause character in stead of the text.
- The beat indicator would be a soft faded highlight on the BPM number, not as bright, but still yellow.
