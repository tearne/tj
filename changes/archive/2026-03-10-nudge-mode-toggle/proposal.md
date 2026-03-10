# Proposal: Nudge Mode Toggle
**Status: Draft**

## Intent
The `c`/`d` keys currently jump the playhead by a fixed 10ms. The `,`/`.` keys apply a temporary ±10% speed warp. Both serve the same broad purpose (fine position correction) but feel different in use. Rather than requiring the user to pick one permanently, allow toggling between the two behaviours on `c`/`d` with a single keypress, and retire `,`/`.`.

## Specification Deltas

### ADDED
- **Nudge mode**: a persistent toggle with two states:
  - `jump` — `c`/`d` seek the playhead ±10ms per press/repeat. While paused, moves the transport position by ±10ms.
  - `warp` — `c`/`d` apply a continuous ±10% speed offset while held; releasing returns to normal speed. While paused, drifts the transport position at ±10% of normal playback speed for as long as the key is held (matching current `,`/`.` paused behaviour).
- `C` / `D` (Shift+C or Shift+D) toggles the nudge mode. Either key has the same effect.
- The active nudge mode is shown in the info bar (e.g. `nudge:jump` or `nudge:warp`).

### REMOVED
- `,` (nudge backward) and `.` (nudge forward) dedicated keys — their behaviour is subsumed by `c`/`d` in `hold` mode.

### MODIFIED
- Info bar: adds a nudge mode indicator.
- Help popup: reflects new keys and mode description; removes `,`/`.` entries.

## Scope
- **Out of scope**: persisting the nudge mode between sessions — deferred to the keyboard-mapping / config file proposal.
