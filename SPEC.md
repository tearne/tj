# Specification: tj

## Overview
A terminal-based music player written in Rust, with a real-time waveform visualisation and beat-aware transport controls.

## Usage

### Launching
```
tj [path]
```
- If `path` is an audio file, opens and begins playing it immediately.
- If `path` is a directory, opens the file browser rooted at that directory.
- If `path` is omitted, opens the file browser rooted at the current working directory.

### File Browser Controls
| Key | Action |
|-----|--------|
| `↑` / `↓` | Move cursor (skips non-audio files) |
| `Enter` | Navigate into directory / load and play audio file |
| `←` / `Backspace` | Go to parent directory |
| `Esc` | Return to player (if one is active) |
| `q` | Quit |

### Player Controls
| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `+` / `-` | Adjust beat phase offset (10ms steps) |
| `Left` / `Right` | Seek backward / forward (small increment, e.g. 5s) |
| `[` / `]` | Beat jump backward / forward by the current beat unit |
| `1`–`6` | Set beat jump unit (4, 8, 16, 32, 64 beats) |
| `b` | Open file browser |
| `q` | Quit |

> Key bindings are indicative; exact bindings are an implementation concern.

## Behaviour

### File Browser
- Displays all files and subdirectories in the current directory, sorted alphabetically.
- Directories are visually distinguished (e.g. trailing `/`, different colour).
- Compatible audio files (FLAC, MP3, OGG, WAV, AAC, OPUS) are highlighted.
- Non-audio files are shown but cannot be selected or navigated into.
- A header shows the current directory path.
- Selecting an audio file dismisses the browser and begins playback.
- The browser can be opened from the player at any time with `b`, rooted at the directory of the currently playing file. Audio continues playing while the browser is open. Pressing `Esc` returns to the player view; selecting a new file loads and plays it.

### Playback
- Supports audio formats: FLAC, MP3, OGG Vorbis, WAV, AAC, OPUS.
- Decode runs on a background thread. A loading screen displays a progress bar showing decode progress.
- Playback begins as soon as decode completes, before BPM analysis is finished.
- Displays track metadata: title, artist, album, duration, current position.

### Beat Detection
- BPM is auto-detected from the audio on load, assuming a constant tempo throughout the track. Hash computation and BPM detection run on a background thread after decode; playback starts immediately with a 120 BPM placeholder.
- While BPM analysis is in progress, beat markers are suppressed, the beat indicator does not flash, and the BPM line shows an animated indicator (e.g. `BPM: --- [analysing ⠋]`). Beat jump uses the 120 BPM placeholder.
- When analysis completes, the BPM updates, beat markers appear, and beat jump uses the detected tempo.
- The detected BPM is rounded to the nearest integer.
- A beat phase offset (in milliseconds) can be adjusted at runtime to align the beat indicator with the audio. The offset and BPM are displayed in the UI.
- The user can correct an inaccurate detection at runtime:
  - `h` halves the BPM; `H` doubles it. Takes effect immediately.
  - `r` re-runs detection cycling through modes: `auto` (default tempogram), `fusion` (tempogram + legacy in parallel), `legacy` (autocorrelation + comb filter). Non-blocking: returns immediately, shows the animated indicator while detection runs in the background.
  - Corrections are persisted to the cache immediately.
- Detected BPM and phase offset are cached in `~/.local/share/tj/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format.
- Each cache entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- On quit, the current phase offset is persisted to the cache.

### Beat Indicator
- A visual indicator flashes on each beat in real time, derived from the detected BPM, playback position, and phase offset.

### Waveform Visualisation
- Two waveform views are displayed simultaneously:
  - **Overview**: full-track waveform, with a playhead marker showing current position.
  - **Detail**: zoomed-in waveform centred on the playhead, with variable zoom level.
- Both views update in real time during playback.
- The detail view tracks the playhead as the track progresses.
- Zoom level for the detail view is adjustable by the user.
- The overview displays a bar marker (every 4 bars) as a full-height line drawn beneath the waveform, visible only in the gaps.
- The detail view displays a beat marker at each beat position as a full-height line drawn beneath the waveform, visible only in the gaps.
- Both sets of markers shift immediately when the phase offset is adjusted.

### Rendering
The following principles are required to achieve smooth, stable rendering:

- **Smooth display position**: The position used for all visual rendering (detail viewport, beat markers) advances by wall-clock elapsed time rather than reading the audio output position directly. The audio output position advances in bursts as the output device requests audio buffers; using it directly causes visible periodic jumps in the display. The smooth position resyncs to the real position if it drifts by more than a small threshold (e.g. 0.5 s), covering seek and pause.
- **Consistent position**: Beat marker columns must be computed from the same smooth display position as the waveform viewport. Using different position sources causes markers to oscillate relative to the waveform.
- **Waveform computation off the UI thread**: Braille dot rasterisation runs on a background thread. The UI thread performs only lightweight per-frame work (colour assignment, span construction) to stay within the frame budget.
- **Stable buffer between recomputes**: The background thread pre-renders a buffer wider than the visible area. The UI thread slides a viewport through this buffer each frame. This avoids recomputing the waveform on every frame tick and prevents ratatui from receiving a changed grid every frame (which would cause a full widget repaint and visible flicker).

### Beat Jump
- Beat jump moves the playhead backward or forward by a user-selected number of beats: 4, 8, 16, 32, 64, or 128.
- The jump is by exactly N × beat_period seconds from the current position, preserving rhythmic continuity.
- Jumping backward past the start clamps to position 0. Jumping forward past the end is a no-op.
- Seeking is implemented via an atomic position counter shared with the audio thread; the audio thread never pauses.
- A ~6ms fade-out before the cut and ~6ms fade-in after eliminate click artefacts without any perceptible gap.
- The detected BPM and current beat jump unit are displayed in the UI.

### Threading
- Audio decode runs on a background thread with progress reported to the TUI via atomics; the TUI render loop starts immediately and remains responsive during decode.
- Hash computation and BPM detection run on a further background thread after decode, communicating results to the TUI via a channel.
- Audio playback runs on a dedicated thread.
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
- Cover art display.
- Volume control, shuffle, repeat.
- Multiple file / queue management.

## Verification
- Launching with no argument opens the file browser in the current working directory.
- Launching with a directory path opens the file browser rooted there.
- Launching with a valid file path plays the track and renders the TUI.
- Beat indicator flashes at the correct tempo, aligned with the audio.
- Phase offset adjustment shifts the flash timing immediately.
- Waveform overview renders the full track immediately after load.
- Detail view updates position in real time during playback without visible lag.
- Beat jump moves playback position by the correct number of beats at the detected BPM.
- All supported formats play without error.
- Quitting cleanly exits without errors or leftover terminal state.
