# Proposal: Audio Latency Calibration
**Status: Ready for Review**

## Intent
The waveform display and beat markers are anchored to the position reported by the audio device — what has been *sent* to the output buffer, not what is currently *heard*. The difference (audio output latency) is typically 20–150ms. After precise BPM calibration this offset becomes clearly perceptible: the display appears to lead the audio. This proposal allows the user to measure and compensate for their system's audio output latency.

## Specification Deltas

### ADDED
- An `audio_latency_ms` value shifts all visual rendering backward by a fixed number of milliseconds. The effective display position is `smooth_display_samp - audio_latency_ms * sample_rate / 1000`. Affects the waveform viewport, beat markers, and beat flash.
- `~` opens and closes calibration mode. While active:
  - A synthetic click tone fires at 120 BPM, injected directly into the mixer.
  - A calibration pulse marker travels through the detail waveform at the same 120 BPM tempo, scrolling toward the playhead centre. The marker is visually distinct from normal beat tick marks (e.g. brighter / different colour).
  - When a pulse marker coincides with the playhead centre, the playhead flashes.
  - `+` / `-` (neither requiring Shift) adjust `audio_latency_ms` in 1ms steps.
  - The user adjusts until the playhead flash and the heard click are simultaneous.
  - Normal playback and all other controls continue to function while calibration is active.
  - The current `audio_latency_ms` value is shown in the info bar.
  - Pressing `~` or `Esc` exits calibration mode; the value is persisted immediately.
- `audio_latency_ms` is stored as a single global value in the cache, separate from per-track entries.
- The `[?]` help overlay lists `~` as the calibration key.

### MODIFIED
- `+` / `-` adjust beat phase offset in normal mode, and `audio_latency_ms` in calibration mode. Neither requires Shift — the increase key is bound to `=` by default (unshifted, adjacent to `-`), with `+` as an additional alias.
