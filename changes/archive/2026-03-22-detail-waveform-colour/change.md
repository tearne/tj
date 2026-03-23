# Detail Waveform Colour
**Type**: Spike
**Status**: Archived

## Goal

Explore colouring the detail waveform to convey frequency content — specifically bass vs treble — giving the user spectral information at a glance without adding a separate view.

## Approach

Compute a **spectral centroid** per column: a short FFT (e.g. 512 samples) over each column's time window, weighted-average the frequency bins to a single value, then map that to a colour gradient (e.g. warm/red for bass-heavy, cool/cyan for treble-heavy).

Each braille cell span in the detail waveform is independently coloured using ratatui's per-span styling. The existing waveform structure already renders column-by-column, so colour can be injected at that point.

## Questions to Answer

1. Does the spectral centroid per column produce a visually interesting and musically meaningful result?
2. Is the FFT cost acceptable at typical zoom levels (~30–80 columns)?
3. What colour gradient reads well in a terminal?
4. Does it conflict visually with the cue marker, tick marks, or playhead?

## Findings

Spectral colouring works well and no FFT is needed. An IIR low-pass (250 Hz, first-order) computed directly on the raw mono samples per column gives accurate bass/treble separation. The existing `SPECTRAL_PALETTES` system carries over unchanged.

Key discoveries during the spike:
- The original `diff_energy` heuristic in `WaveformData` caused bass colour to appear late (it measured the transient attack, not the sustained low-frequency content). Replaced with the IIR approach, which fixed the timing.
- Per-column IIR (resetting state at each column boundary) causes sharp colour transitions at wide zoom, producing flickering. Fixed with a box-smooth (radius 3) over the computed `bass_ratio` and `peak_amp` arrays.
- Amplitude brightness (scaling colour by `0.15 + 0.85 * peak_amp.sqrt()`) adds a useful dynamics dimension with no extra cost.
- Coloured tick marks were tried and rejected — they competed visually with the waveform; kept them gray.
