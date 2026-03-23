# Waveform Palettes
**Type**: Spike
**Status**: Done

## Goal

Explore richer colour palettes for the waveforms, and per-deck palette schemes so the two decks are visually distinct at a glance.

The current system maps bass→treble linearly across two RGB endpoints. This spike explores:
- Three-stop gradients (e.g. sub-bass / mid / high mapped to three colours) for more expressive spectral reads
- Paired palette schemes where Deck A and Deck B use complementary or contrasting hues, making it immediately clear which waveform belongs to which deck
- Whether per-deck palettes should be independent (user cycles each separately) or paired (selecting a scheme sets both decks at once)

## Questions to Answer

1. Does a three-stop gradient read better than two-stop at typical terminal sizes?
2. What pairing schemes work well together without either deck dominating visually?
3. Should the palette data structure change (e.g. add a third stop), or can three-stop be approximated by blending two two-stop palettes?
4. Should pairing be automatic (load assigns complementary palettes) or manual?

## Findings

See `findings.md`.
