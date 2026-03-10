# Proposal: Waveform Outline Mode
**Status: Draft**

## Intent
The current waveform renders as a filled silhouette — all braille dots from the centre outward to the peak amplitude are set. An outline mode would instead set only the outermost dots at each column, drawing the waveform envelope as a thin line rather than a solid shape. This may be more legible at certain zoom levels and is worth experimenting with as an alternative visual style.

## Specification Deltas

### ADDED
- A waveform render mode toggle with two states:
  - `fill` (default) — current behaviour: all dots from centre to peak are set.
  - `outline` — only the dot at the peak amplitude is set per column, drawing the waveform as a thin outline.
- A dedicated key (unmapped by default) toggles between `fill` and `outline`. A new mappable function `waveform_style` is added to `config.toml`.
- The active render mode is not indicated in the UI (experimental feature; no display real estate allocated until the mode proves useful).

## Scope
- **In scope**: outline rendering for the detail waveform only.
- **Out of scope**: applying to the overview; persisting the mode between sessions; additional render styles beyond these two.
