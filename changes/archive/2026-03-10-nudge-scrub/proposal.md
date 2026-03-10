# Proposal: Nudge Scrub
**Status: Ready for Review**

## Intent
When the transport is paused, nudging (`c`/`d`) moves the playhead silently. This makes it hard to judge alignment by ear — the user must play, listen, pause, and re-nudge repeatedly. Playing a brief audio snippet at the new position after each nudge step gives immediate aural feedback, making beat-phase alignment much faster.

## Specification Deltas

### ADDED
- While paused, each nudge step (in either mode) plays a short audio snippet starting at the new playhead position. The snippet duration matches one screen column at the current zoom level (i.e. the same time span represented by one braille character in the detail view).
- If a snippet is already playing when the next nudge fires, the current snippet is cut and replaced immediately by a new one from the updated position. There is no queuing.

## Scope
- **In scope**: scrub on nudge (`c`/`d`) while paused, both jump and warp modes.
- **Out of scope**: phase offset adjustment (`+`/`-`) — the playhead doesn't move so the audio would be identical on every press; visual tick-mark feedback is sufficient.
