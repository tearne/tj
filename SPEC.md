# Specification: tj

## Overview
A terminal-based music player written in Rust, with a real-time waveform visualisation and beat-aware transport controls.

## Usage

### Launching
```
tj <file>
```
Opens and begins playing the specified audio file.

### Keyboard Controls
| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `+` / `-` | Adjust beat phase offset (10ms steps) |
| `Left` / `Right` | Seek backward / forward (small increment, e.g. 5s) |
| `[` / `]` | Beat jump backward / forward by the current beat unit |
| `1`–`6` | Set beat jump unit (4, 8, 16, 32, 64 beats) |
| `q` | Quit |

> Key bindings are indicative; exact bindings are an implementation concern.

## Behaviour

### Playback
- Supports audio formats: FLAC, MP3, OGG Vorbis, WAV, AAC, OPUS.
- Begins playback immediately on launch.
- Displays track metadata: title, artist, album, duration, current position.

### Beat Detection
- BPM is auto-detected from the audio on load, assuming a constant tempo throughout the track.
- The detected BPM is rounded to the nearest integer.
- A beat phase offset (in milliseconds) can be adjusted at runtime to align the beat indicator with the audio. The offset and BPM are displayed in the UI.

### Beat Indicator
- A visual indicator flashes on each beat in real time, derived from the detected BPM, playback position, and phase offset.

### Waveform Visualisation
- Two waveform views are displayed simultaneously:
  - **Overview**: full-track waveform, with a playhead marker showing current position.
  - **Detail**: zoomed-in waveform centred on the playhead, with variable zoom level.
- Both views update in real time during playback.
- The detail view tracks the playhead as the track progresses.
- Zoom level for the detail view is adjustable by the user.

### Beat Jump
- Beat jump moves the playhead backward or forward by a user-selected number of beats: 4, 8, 16, 32, or 64.
- The detected BPM and current beat jump unit are displayed in the UI.

### Threading
- Audio decode and playback run on a dedicated thread.
- TUI rendering runs on a separate thread.
- State is shared between threads via lock-free or minimal-contention primitives to meet real-time rendering requirements.

## Constraints
- Implementation language: Rust.
- TUI framework: `ratatui`.
- Audio decoding: `symphonia`.
- Audio playback: `rodio`.
- BPM detection: `stratum-dsp`.
- Target platform: Linux (primary); other Unix-like systems are a stretch goal.

## Out of Scope (deferred)
- Directory browser and playlist support.
- Cover art display.
- Volume control, shuffle, repeat.
- Multiple file / queue management.

## Verification
- Launching with a valid file path plays the track and renders the TUI.
- Beat indicator flashes at the correct tempo, aligned with the audio.
- Phase offset adjustment shifts the flash timing immediately.
- Waveform overview renders the full track immediately after load.
- Detail view updates position in real time during playback without visible lag.
- Beat jump moves playback position by the correct number of beats at the detected BPM.
- All supported formats play without error.
- Quitting cleanly exits without errors or leftover terminal state.
