# Filter Slope
**Type**: Proposal
**Status**: Draft

## Intent

Add per-deck filter slope control so the rolloff steepness can be adjusted at runtime. Steeper slopes cut more aggressively; shallower slopes let frequencies bleed through more gradually.

## Keys

| Key | Action |
|-----|--------|
| `&` (Shift+7) | Deck 1 slope increase |
| `U` (Shift+u) | Deck 1 slope decrease |
| `*` (Shift+8) | Deck 2 slope increase |
| `I` (Shift+i) | Deck 2 slope decrease |

Sits naturally on the same physical keys as the filter sweep controls.

## Slope Steps

Likely options: 1-pole (6 dB/oct), 2-pole (12 dB/oct), 3-pole (18 dB/oct), 4-pole (24 dB/oct). Default: 2-pole (current behaviour). TBD whether all four are useful or just two (12/24).

## Open Question: Visual

Need a single character (or small cluster) to communicate slope steepness in the filter section of the info bar. Candidates:

- **Order number** — `₁` `₂` `₃` `₄` (subscript) or plain `2` / `4`
- **Slope character** — a diagonal suggesting the rolloff angle; steeper = more vertical. Unicode has `╱` (shallow) but nothing obviously steeper in a single glyph.
- **dB/oct label** — `12` / `24` but two characters may be awkward

## Questions

- How many slope steps? 2 (12/24) or 4 (6/12/18/24)?
- Should slope persist to cache?
- Visual TBD.
