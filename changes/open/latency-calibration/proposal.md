# Proposal: Audio Latency Calibration
**Status: Note**

## Problem
The waveform display and beat markers are anchored to the position reported by the audio device — what has been *sent* to the output buffer, not what is currently *heard* through the speakers. The difference (audio output latency) is typically 20–150ms depending on the system and ALSA buffer configuration. After precise BPM calibration this offset becomes clearly perceptible: the display appears to lead the audio.

## Intent
Allow the user to measure and compensate for their system's audio output latency, so the waveform and beat markers align with what is actually heard rather than what has been sent to the device.

## Approach sketch

### Runtime offset
Introduce an `audio_latency_ms` value that shifts the display position backward by a fixed number of milliseconds. All visual rendering (waveform viewport, beat markers, beat flash) uses `smooth_display_samp - audio_latency_ms * sample_rate / 1000` as its effective position.

### Calibration mode
A dedicated calibration mode makes it easy to find the right value without trial and error:
- A repeating click (audio) fires at a fixed metronome interval.
- A visual flash fires at the same interval, driven by the *reported* position.
- The user adjusts `audio_latency_ms` (e.g. with `+`/`-`) until the flash appears simultaneous with the click they hear.
- The latency value is persisted to the cache (not the config — it is device-specific, not a user preference).

## Open questions
- Which key triggers calibration mode?
- Should the metronome click be a synthetic tone injected into the mixer, or a visual-only indicator with a real audio click derived from the track?
- Should calibration persist per-device (keyed on audio device name) or as a single global value?
