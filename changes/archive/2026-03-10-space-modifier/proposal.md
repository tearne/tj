# Proposal: Space as Modifier Key
**Status: Draft**

## Intent
Repurpose `Space` from a direct play/pause action into a held modifier. Holding `Space` while pressing another key activates a secondary action. This frees up the most ergonomic key for combinations and reduces the chance of accidental play/pause.

## Specification Deltas

### ADDED
- **Space modifier**: holding `Space` activates a modifier state. `Space` alone (press and release without another key) has no effect.
- `Space+Z` — play / pause (replaces bare `Space`).
- `Space+F` or `Space+V` — reset playback tempo to the detected BPM (`bpm` → `base_bpm`, speed → 1×). Useful for returning to normal after a `f`/`v` adjustment.

### REMOVED
- Bare `Space` as play/pause.

### MODIFIED
- The `play_pause` mappable function key changes from `space` to `space+z` in the default config.
- A new mappable function `tempo_reset` is added, bound to `space+f` and `space+v` by default.
- The config key string format is extended to support `space+<key>` chords.

## Scope
- **In scope**: `Space` modifier state; `space+z` play/pause; `space+f`/`space+v` tempo reset; config format extension for `space+` chords.
- **Out of scope**: other modifier chords (e.g. `alt+`, `ctrl+` user-defined bindings); Space modifier visible in the UI.

## Unresolved
- Should `Space` alone still trigger play/pause as a fallback (if released without a chord), or do nothing? Proposed: do nothing, to avoid accidental triggers.
