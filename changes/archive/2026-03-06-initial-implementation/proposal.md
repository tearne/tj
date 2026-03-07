# Proposal: Initial Implementation
**Status: Approved**

## Intent
Validate beat detection before investing in the full player UI. A minimal spike that plays audio and flashes a visual indicator on each detected beat, allowing the reliability of BPM auto-detection to be evaluated across a range of tracks.

## Specification Deltas

### ADDED
- `tj <file>` opens and plays a FLAC audio file.
- BPM is auto-detected from the audio on load.
- TUI displays the detected BPM value.
- TUI displays a beat indicator that flashes on each beat in real time.
- Play/pause control via keyboard.

## Scope
- **In scope**: FLAC playback, BPM auto-detection, beat flash indicator, play/pause, minimal TUI.
- **Out of scope**: all other formats, waveform views, seek, beat jump, metadata display, cover art, browser/playlists.
