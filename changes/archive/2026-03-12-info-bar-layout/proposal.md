# Proposal: Info Bar Layout Revision
**Status: Ready for Review**

## Intent
Stabilise the info bar so that variable-width fields (BPM adjustment, tap count, filter indicator) no longer cause the spectrum strip to shift horizontally. Rationalise the field set and rename `volume` to `level`.

## Specification Deltas

### MODIFIED
- The info bar is split into a **left group** (left-aligned) and a **right group** (right-aligned, filling remaining width):
  - **Left**: play/pause icon, BPM, phase offset.
  - **Right**: `nudge:jump` / `nudge:warp`, zoom indicator, level, filter indicator (when active), spectrum strip.
- The nudge mode field uses a fixed-width format (`nudge:jump` / `nudge:warp`) so toggling between modes does not shift any field to its right.
- The zoom field is labelled explicitly: `zoom:4s` (was `4s`).
- Volume is renamed to **level** throughout: displayed as `level:80%` (was `vol:80%`), and the config keys `volume_up` / `volume_down` are renamed to `level_up` / `level_down`.
- The palette name is removed from the info bar.
- The spectrum strip remains the rightmost element of the right group, held at a stable position regardless of other field changes.

### REMOVED
- Palette name from the info bar (still cycled with `p`; active palette visible via waveform colour only).

## Scope
- **In scope**: info bar field set, layout split, nudge fixed width, zoom label, volume→level rename.
- **Out of scope**: transient overlays, per-field animations, hiding fields at narrow terminal widths.
