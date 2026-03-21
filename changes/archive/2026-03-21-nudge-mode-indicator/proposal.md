# Proposal: Nudge Mode Indicator on Detail Info Bar
**Status: Archived**

## Overview

The nudge mode (Jump / Warp) is currently shown only in each deck's info line. When no
deck is loaded, the mode is invisible. Even when decks are loaded, the info line is dense
and the mode label can be overlooked. The detail info bar already carries shared state
(zoom, latency, `[SPC]`) and is the right place for this indicator.

## Behaviour

Append `[JUMP]` or `[WARP]` to the detail info bar depending on the active nudge mode.

```
  zoom:4s  lat:12ms  [JUMP]
  zoom:4s  lat:12ms  [WARP]
  zoom:4s  lat:12ms  [WARP]  [SPC]
```

`[SPC]` appears to the right of the nudge indicator so that the nudge label stays in a
fixed position and does not jump when the Space modifier is activated.

## Implementation Notes

- `nudge_mode` is per-deck but both decks are always toggled together (`NudgeModeToggle`
  sets the mode on all loaded decks). Reading from the first loaded deck is sufficient.
- If no deck is loaded, default to `NudgeMode::Jump` for display purposes.
- The detail info bar format string gains a `[JUMP]`/`[WARP]` segment alongside the
  existing `[SPC]` segment.
