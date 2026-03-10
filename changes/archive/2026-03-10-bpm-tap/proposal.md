# Proposal: BPM Tap Detection
**Status: Draft**

## Intent
Auto-detected BPM can be imprecise on complex material. Letting the user tap `b` in time with the beat provides a fast, intuitive way to correct both tempo and phase alignment simultaneously.

## Specification Deltas

### ADDED
- `b` is a tap key. Pressing it in time with the beat accumulates a rolling tap history.
- A tap session resets automatically if more than 2 seconds elapses between taps.
- After 8 or more taps in a session, `base_bpm` and `offset_ms` are updated live on each subsequent tap. Fewer than 8 taps are accumulated silently (no update applied) to allow the user to settle into the rhythm before corrections begin.
- BPM is derived from the median inter-tap interval. Phase offset is derived from the tap timestamps relative to the computed beat grid.
- `bpm` (playback speed) is not affected — any active `f`/`v` adjustment stays in place, now relative to the corrected `base_bpm`.
- The tap count is shown in the info bar while a session is active (e.g. `tap: 5`), disappearing when the session resets.

### MODIFIED
- `open_browser` default binding changes from `b` to `space+a`.

## Scope
- **Out of scope**: browser exit key behaviour (separate proposal); tap visualisation beyond the info bar count.
