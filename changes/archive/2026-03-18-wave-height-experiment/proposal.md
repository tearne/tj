# Experiment: Wave Height
**Status: Concluded — v0.5.100**

## What Was Tried

Explored reducing vertical space consumed by the two detail waveforms and their tick rows.

### Overview height (kept)
`OV_MAX` reduced from 4 → 3 rows. Saves 2 rows across both overviews..

### Detail height (reverted)
`detail_height` default reduced from 6 → 5 in config. Reverted — no clear benefit once tick row work is addressed properly.

### Tick row experiments (all reverted)
Several approaches were tried to reduce the 2 tick rows per deck (4 rows total):

1. **Single centre tick row** — one dedicated row at the vertical centre of each detail panel. Stable, saved 1 row per deck, but felt visually off.
2. **Colour-only ticks in waveform** — no dedicated row; tick columns recoloured in the waveform. Flickered due to sub-column scrolling: colour assignment uses integer column indices while the waveform advances at half-column resolution, causing the colour to oscillate relative to the waveform content every frame.
3. **OR'd tick dots in waveform** — tick braille bytes OR'd into the waveform at the centre row. Same jitter problem: the tick byte (left vs right dot column) doesn't go through the `shift_braille_half` pipeline that the waveform does, so dots oscillate relative to the waveform at wide zoom.
4. **Inner-edge tick rows** — Deck 1 shows only its bottom tick row; Deck 2 shows only its top tick row, placing the two rows adjacent in the middle. Stable (dedicated rows, no sub-column issue), but the shared-tick-row proposal does this more efficiently.

## Conclusion

The colour-only and OR approaches cannot be made stable without routing the tick marks through the same sub-column pipeline as the waveform — which requires encoding them in the BrailleBuffer, currently ruled out for isolated marks (see SPEC/waveforms.md, *Rendering*).

The correct path forward is the **shared tick row** proposal (`changes/open/shared-tick-row/`): a single row between the two decks renders both grids via OR, using dedicated-row rendering (stable) and saving 3 rows overall.

## Net Change at Conclusion

- `OV_MAX`: 4 → 3 (kept)
- `detail_height` default: unchanged (6)
- Tick rows: unchanged (2 per deck, top and bottom)
