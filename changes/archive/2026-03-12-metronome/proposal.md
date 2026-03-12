# Proposal: Metronome
**Status: Draft**

## Intent
When adjusting `offset_ms` to align beat markers with the music, there is no audio feedback — the user must watch the visual beat flash and listen simultaneously. A metronome mode fires click tones in time with the current BPM and phase offset, giving direct auditory feedback to make offset tuning faster and more intuitive.

## Specification Deltas

### ADDED
- A metronome mode that fires click tones in time with the active BPM and current `offset_ms`.
- `'` toggles metronome mode on and off.
- While metronome is active, a `♪` (U+266A) symbol is shown in red immediately after the BPM value in the info bar. No indicator when inactive.
- The click timing updates immediately when `offset_ms` or BPM changes, so the user hears the effect of each adjustment in real time.
- Metronome mode survives pause, `offset_ms` adjustments, and BPM changes within the same track. It resets to off on track load.
- The click tone reuses the calibration click sound.

### MODIFIED
- The `[?]` help overlay lists `'` as the metronome toggle.
