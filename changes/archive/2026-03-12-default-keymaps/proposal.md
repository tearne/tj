# Proposal: Update Default Key Mappings
**Status: Ready for Review**

## Intent
Rationalise default key bindings: improve ergonomics, free up keys, and remove the standalone BPM redetect action.

## Specification Deltas

### MODIFIED
- `zoom_in` / `zoom_out`: `=` / `-` (was `Z` / `z`)
- `offset_increase` / `offset_decrease`: `+` / `_` (Shift+= / Shift+-) (was `=`/`+` and `-`)
- `open_browser`: `z` (was `space+a`)
- `level_up` / `level_down`: `j` / `m` (was `up` / `down`)

### ADDED
- `level_max`: `space+j` — set level to 100% immediately
- `level_min`: `space+m` — set level to 0% immediately
- `terminal_refresh` (`` ` ``): force a full terminal clear and redraw to recover from display glitches

### REMOVED
- `bpm_redetect` (`t`) — tap BPM (`b`) is sufficient; standalone redetect no longer needed.
- All code for the `t` cycling re-detection modes (`auto`, `fusion`, `legacy`).

### CLARIFIED (spec documentation)
- The `b` tap session triggers a background re-detection using the **legacy autocorrelation** algorithm, constrained to ±5% of the tapped BPM at 0.1 BPM resolution. The tap resolves octave ambiguity; the analyser provides sub-integer precision. "Fusion" (tempogram + legacy in parallel) was only ever used by the now-removed `t` action and is no longer referenced.

