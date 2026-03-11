# Proposal: Spectrum Analyser
**Status: Ready for Review**

## Intent
Display a compact real-time frequency spectrum in the player view, giving the user a quick read of spectral content without occupying significant screen space. The display is intentionally minimal — one braille character tall and eight characters wide — to sit comfortably in the existing layout.

## Specification Deltas

### ADDED
- A spectrum analyser strip is displayed in the player view, one braille row tall (4 dot rows) and 8 braille characters wide (16 frequency bins).
- Each braille character encodes two adjacent frequency bins as a 2-column, 4-row bar chart. Bin height is quantised to 0–4 dot rows. The 16 bins cover the audible range on a logarithmic scale (approximately 20 Hz – 20 kHz).
- The spectrum is updated twice per beat (every half beat-period). Between updates the display holds its last value.
- The analyser operates on a short FFT window taken from the decoded audio at the current playback position. Window length is an implementation detail, chosen to give adequate frequency resolution within the update budget.
- While BPM analysis is still in progress (beat period unknown), the spectrum updates on a fixed 500ms fallback interval.
- The strip is positioned in the info bar line, appended after the existing fields. If the terminal is too narrow it is omitted gracefully.
- The analyser is always active while a track is loaded; there is no on/off toggle.

### MODIFIED
- The info bar gains a spectrum strip field at its right end.

## Scope
- **In scope**: log-scale FFT, 16-bin display, beat-rate update, mono mix of decoded samples.
- **Out of scope**: peak hold, configurable bin count, colour-coded bins, per-channel display, freeze/hold control.
