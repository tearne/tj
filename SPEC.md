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
  - **Detail view**: zoomed waveform centred on the playhead, with variable zoom level.
- Both views update in real time during playback.
- The Detail view tracks the playhead as the track progresses.
- Zoom level for the Detail view is adjustable by the user.
- The Overview displays bar markers as full-height lines drawn beneath the waveform, visible only in the gaps. The marker interval starts at every 4 bars and doubles if there are fewer than two characters between any pair of adjacent markers, repeating until all pairs have at least two characters between them. A legend in the top-right corner of the Overview shows the current interval (e.g. `4 bars`, `8 bars`).
- The detail view displays a beat marker at each beat position as a full-height line drawn beneath the waveform, visible only in the gaps.
- Buffer columns representing sample positions before the start of the track render as silence (zero amplitude), not as a mirror of position 0.
- Both sets of markers shift immediately when the phase offset is adjusted.

### Rendering
The following principles are required to achieve smooth, stable rendering:

- **Smooth display position**: The position used for all visual rendering (detail viewport, beat markers) advances by wall-clock elapsed time rather than reading the audio output position directly. The audio output position advances in bursts as the output device requests audio buffers; using it directly causes visible periodic jumps. Small accumulated drift (>20 ms) is corrected gradually. Large drift (e.g. after seek or startup) is snapped to the nearest **column-grid** boundary — not to the raw sample position — ensuring `sub_col = false` immediately after every seek.
- **Consistent position**: Beat marker columns must be computed from the same **smooth display position** as the waveform viewport. Using different position sources causes markers to oscillate relative to the waveform.
- **Waveform computation off the UI thread**: Braille dot rasterisation runs on a background thread, producing a buffer in **buffer space**. The UI thread performs only lightweight per-frame work (translating to **screen space**, colour assignment, span construction) to stay within the frame budget.
- **Stable buffer between recomputes**: The background thread pre-renders a buffer wider than the visible area. The UI thread slides a viewport through it each frame, avoiding a full recompute on every tick. Passing an unchanged buffer to ratatui prevents a full widget repaint and visible flicker.
- **Background thread uses smooth position**: The background thread must use the **smooth display position** as both the drift trigger and the buffer centre. Using the raw audio position causes premature recomputes on audio bursts and centres the buffer at a different position than the viewport expects, producing a visible jump on each recompute.
- **Fixed column grid**: The **anchor** is aligned to the **column grid** and peaks are computed by direct per-column indexing. This ensures any two buffers at the same zoom level share identical cell boundaries, so overlapping cells are byte-for-byte equal and the buffer handoff is visually seamless.
- **Early recompute trigger**: The background thread begins computing a new buffer when drift reaches 3/4 of the screen width (not at the edge), ensuring the new buffer is ready before the old one runs out. A last-valid-viewport fallback prevents black frames in the rare case the OS delays the background thread.
- **Tick marks in screen space**: Beat tick marks must be computed in **screen space** from the **quantised viewport centre**, not encoded as isolated marks in **buffer space**. Isolated marks in buffer space produce completely different braille characters on alternating frames when processed through the half-column shift, causing visible oscillation at wide zoom.
- **Consistent tick and viewport centre**: Tick mark positions and the waveform viewport must both be derived from the **quantised viewport centre**, not from the raw smooth display position. The two can differ by up to half a column, causing ticks to snap relative to the waveform on every frame at wide zoom.

The detail waveform height is user-adjustable at runtime with `{` (decrease) and `}` (increase), defaulting to 8 rows. Any unused space below the panel is left blank. The current height is shown in the key hints line.

The detail waveform scrolls at half-column resolution: the viewport can be positioned at half-character offsets without modifying the pre-rendered buffer (see *Glossary — Half-column scrolling*).

The render frame period adapts to the current zoom level and detail panel width, targeting one dot-column advance per frame. At very tight zoom it is capped at ~120 fps; at very wide zoom it is capped at ~5 fps to keep input responsive.

### Needle Drop
- A left mouse click anywhere on the Overview waveform seeks the transport to the start of the nearest bar marker at or to the left of the click position. Playback state is preserved — if playing, playback continues from the new position; if paused, the transport remains paused. The Detail view recentres on the new position immediately.

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

## Glossary

### The rendering pipeline

Waveform rendering proceeds in two stages:

```
Audio samples ──[background thread]──▶ Braille buffer ──[UI thread]──▶ Screen
  (sample space)                        (buffer space)               (screen space)
```

The background thread rasterises peaks into a buffer wider than the screen. The UI thread slides a viewport through the buffer each frame, applying a half-column shift when needed, and passes the result to the terminal.

**Sample space**
The coordinate of raw audio data. A position is an integer sample index from 0 (start of track) to `total_samples − 1`.

**Column grid**
A coordinate system that partitions the timeline into discrete character-column cells, each `samples_per_col` samples wide. Cell `n` spans `[n × samples_per_col, (n+1) × samples_per_col)`. The grid is unbounded — cells extend before sample 0, which is what allows pre-track cells to render as silence rather than clamping to the first sample. Any two buffers computed at the same zoom level share identical cell boundaries wherever they overlap, making overlapping cells byte-for-byte equal.

**Buffer space**
The coordinate of the pre-rendered braille byte buffer: an array of cells indexed 0 to `buf_cols − 1`, each corresponding to one column-grid cell. The buffer is wider than the screen and centred on the **anchor** — the column-grid cell nearest the current playhead position. Elements that must appear in screen space (such as beat tick marks) should not be computed in buffer space: the half-column shift transforms isolated marks into different braille characters on alternating frames, causing visible oscillation.

**Screen space**
The coordinate of visible screen columns, indexed 0 (left edge) to `dw − 1` (right edge). The playhead is fixed at `centre_col`. At half-column resolution, positions are expressed in half-character units — even values are the left half of a character, odd values the right half — so that tick marks can be placed between character boundaries.

### Half-column scrolling

Each braille character encodes a 2×4 dot grid. By combining the right dot-column of one buffer cell with the left dot-column of the next, the viewport can be positioned at half-character offsets without modifying the buffer:

```
Buffer:   │  cell[n]  │  cell[n+1]  │
          │ left│right│ left│right  │

sub_col=false → screen column shows cell[vs+c]
sub_col=true  → screen column shows right(cell[vs+c]) + left(cell[vs+c+1])
                 ╰──────────────────────────────╯
                        shift_braille_half
```

`sub_col` flips each time the smooth display position crosses a half-column boundary, advancing the viewport by one dot-column per flip.

### Rendering positions

**Smooth display position**
The sample position used as the rendering playhead. It advances by wall-clock elapsed time rather than from the audio output position (which advances in bursts). After a large drift — on seek or startup — it snaps to the nearest column-grid boundary, ensuring `sub_col = false` immediately after a seek. The single source of truth for all rendering.

**Quantised viewport centre**
The smooth display position rounded to the nearest half-column boundary. Both the waveform viewport and beat tick marks must be derived from this value — not from the raw smooth display position, which can differ by up to half a column, causing visible oscillation at wide zoom.

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
