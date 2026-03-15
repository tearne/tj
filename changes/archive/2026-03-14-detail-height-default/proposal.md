# Proposal: Reduce Default Detail Waveform Height
**Status: Draft**

## Intent

A taller detail waveform means more braille characters written to the terminal each frame, increasing output bandwidth and causing jerkiness sooner on slower terminals or connections. Reducing the default height keeps rendering smooth in more environments without changing the user's ability to increase it with `}`.

## Specification Deltas

### ADDED

- **`detail_height` display config key**: sets the initial detail waveform height (in total rows, including the 2-row tick area). Missing key falls back to the default. Adjustable at runtime with `{`/`}` as before.

### MODIFIED

- **Detail waveform default height**: reduced from 8 rows to 6 rows (4 waveform rows + 2 rows for the tick/border area).
