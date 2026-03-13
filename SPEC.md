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
| `Space+Z` | Play / Pause |
| `Space+F` / `Space+V` | Reset tempo to detected BPM (speed → 1×) |
| `+` / `_` | Adjust beat phase offset (10ms steps) |
| `~` | Toggle latency calibration mode (only while paused); `d` / `c` adjust latency while active |
| `u` / `7` | Filter sweep: `u` toward LPF (lower cutoff), `7` toward HPF (higher cutoff) |
| `Space+u` / `Space+7` | Snap filter to flat (bypass) |
| `Left` / `Right` | Seek backward / forward (small increment, e.g. 5s) |
| `1` / `2` / `3` / `4` | Beat jump forward 1 / 4 / 16 / 64 beats |
| `q` / `w` / `e` / `r` | Beat jump backward 1 / 4 / 16 / 64 beats |
| `j` / `m` | Level up / down |
| `Space+J` / `Space+M` | Level 100% / 0% |
| `c` / `d` | Nudge backward / forward (mode-dependent) |
| `C` / `D` | Toggle nudge mode: `jump` (10ms seek) / `warp` (±10% speed) |
| `-` / `=` | Zoom in / out (disabled in calibration mode) |
| `{` / `}` | Detail height decrease / increase |
| `f` / `v` | BPM +0.1 / −0.1 |
| `F` / `V` | Detected BPM +0.01 / −0.01 |
| `?` | Toggle key binding help popup |
| `b` | Tap BPM detection |
| `'` | Toggle metronome |
| `z` | Open / close file browser |
| `` ` `` | Refresh terminal (clear display glitches) |
| `Esc` / `Ctrl-C` | Quit |

> Key bindings reflect the defaults in `config.toml`. All player bindings are user-configurable.

## Behaviour

### File Browser
- Displays all files and subdirectories in the current directory, sorted alphabetically.
- Directories are visually distinguished (e.g. trailing `/`, different colour).
- Compatible audio files (FLAC, MP3, OGG, WAV, AAC, OPUS) are highlighted.
- Non-audio files are shown but cannot be selected or navigated into.
- A header shows the current directory path.
- Selecting an audio file dismisses the browser and begins playback.
- The browser can be opened and closed from the player at any time with `z`. Audio continues playing while the browser is open. Pressing `Esc` returns to the player view; selecting a new file loads and plays it.
- The last visited directory is persisted to the cache between sessions. The browser always opens at the last visited path (falling back to CWD if it no longer exists). If a directory or file argument is given on the command line, it overrides the last visited path for the first browser open of that session only; subsequent opens resume from last visited.

### Playback
- Supports audio formats: FLAC, MP3, OGG Vorbis, WAV, AAC, OPUS.
- When playback reaches the end of the track, the transport pauses and the playhead returns to the start. The player view stays open and fully interactive.
- Decode runs on a background thread. A loading screen displays a progress bar showing decode progress.
- Playback begins as soon as decode completes, before BPM analysis is finished.
- Displays track metadata: title, artist, album, duration, current position.

### Beat Detection
- BPM is auto-detected from the audio on load, assuming a constant tempo throughout the track. Hash computation and BPM detection run on a background thread after decode; playback starts immediately with a 120 BPM placeholder.
- While BPM analysis is in progress, beat markers are suppressed, the beat indicator does not flash, and the BPM line shows an animated indicator (e.g. `BPM: --- [analysing ⠋]`). Beat jump uses the 120 BPM placeholder.
- When analysis completes, the BPM updates, beat markers appear, and beat jump uses the detected tempo.
- The detected BPM is displayed to two decimal places.
- A beat phase offset (in milliseconds) can be adjusted at runtime to align the beat indicator with the audio. The offset and BPM are displayed in the UI.
- `offset_ms` is snapped to the nearest 10 ms boundary on load from the cache, ensuring `+`/`-` steps always land on multiples of 10 ms and 0 ms is always reachable. After each adjustment and on cache load, `offset_ms` is wrapped into `[0, beat_period_ms)` using `rem_euclid`, where `beat_period_ms` is derived from `base_bpm` rounded to the nearest 10 ms, ensuring the offset always remains on the 10 ms grid.
- The user can correct an inaccurate detection at runtime:
  - `f` increases the effective BPM by 0.1; `v` decreases it by 0.1. Adjustments affect playback speed proportionally (relative to the detected BPM) and clamp to the range 40.0–240.0.
  - `b` tap-detects BPM: press in time with the beat. After 8 taps a rolling median of inter-tap intervals sets `base_bpm` and derives `offset_ms` from the tap phase. Any active `f`/`v` speed ratio is preserved relative to the new `base_bpm`. The tap count is shown in the info bar (`tap:N`) while a session is active; tapping stops 2 seconds after the last tap. When the session ends, a background re-detection pass is triggered automatically: the full track is re-analysed using legacy autocorrelation with `bpm_resolution: 0.1`, with the search window narrowed to ±5% of the tapped BPM. This achieves sub-integer precision while using the tap to resolve octave ambiguity. If re-detection returns a result, `base_bpm` is updated to the analyser's value; the tap-derived `offset_ms` is preserved. If the tap session resets before re-detection completes, the in-flight result is discarded.
  - Corrections are persisted to the cache immediately.
- Detected BPM and phase offset are cached in `~/.local/share/tj/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format. The cache also stores the last browser directory.
- Each cache entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- On quit, the current phase offset is persisted to the cache.

### Info Bar
- A single line at the top of the player view. Content is split into two groups separated by a variable-width spacer that fills remaining width, keeping the right group pinned to the right edge regardless of transient field changes:
  - **Left group**: play/pause icon (`▶`/`⏸`), BPM, `♪` in red when metronome is active, phase offset. Tap count (`tap:N`) appended transiently while a tap session is active. In calibration mode the entire info bar is replaced with `lat:Nms  d/c adjust  ~ exit`.
  - **Right group**: nudge mode (`nudge:jump` / `nudge:warp`, fixed width), zoom indicator (`zoom:Ns`), level (`level:▕N▏` — single eighth-block character in dark yellow, in a bracketed indicator with mid-grey brackets), `lat:Xms` (shown only when `audio_latency_ms > 0`), spectrum strip.
- The nudge mode field is always present and fixed-width so toggling between `jump` and `warp` does not shift anything to its right.
- When no tempo adjustment is active, the detected BPM is shown to two decimal places (e.g. `120.00`) and receives a soft amber beat-flash. When a `f`/`v` adjustment is active, the detected BPM is shown plain and the adjusted tempo is shown alongside in parentheses (e.g. `120.00 (124.40)`), with only the adjusted number receiving the beat-flash.
- `F` increases `base_bpm` by 0.01; `V` decreases it by 0.01. Both clamp to 40.0–240.0. Adjusting `base_bpm` resets any active `f`/`v` playback offset (`bpm` is set equal to the new `base_bpm`, speed returns to 1×) and is persisted to the cache immediately.
- Pressing `?` opens a modal key binding reference overlay; any key dismisses it.
- During BPM analysis the BPM field shows an animated spinner.

### Waveform Visualisation
- Two waveform views are displayed simultaneously:
  - **Overview**: full-track waveform, with a playhead marker showing current position.
  - **Detail view**: zoomed waveform centred on the playhead, with variable zoom level.
- Both views update in real time during playback.
- The Detail view tracks the playhead as the track progresses.
- Zoom level for the Detail view is adjustable by the user.
- The Overview waveform is coloured by spectral content: each column blends between an orange/warm colour (bass-heavy) and a cyan/cool colour (treble-heavy), based on the ratio of low-frequency to high-frequency energy in that section. The frequency crossover is ~250 Hz. Several colour palettes are available and cycle with `p`; the active palette name is shown in the info bar.
- The Overview waveform is rendered at half-column braille resolution: each braille character encodes two independent audio columns (left dot column and right dot column), doubling the horizontal detail within the same screen width.
- The Overview displays bar markers as thin vertical lines (`│`) spanning the full overview height. The marker interval starts at every 4 bars and doubles if there are fewer than four characters between any pair of adjacent markers, repeating until all pairs have at least four characters between them. A legend in the top-right corner of the Overview shows the current interval (e.g. `4 bars`, `8 bars`).
- When the remaining playback time falls below a configurable threshold (default 30 seconds), the overview bar markers flash in time with the BPM: the marker colour alternates each beat (one beat on, one beat off) using a muted reddish-grey. The warning is active only during playback. The threshold is configurable via `warning_threshold_secs` in the `[display]` section of `config.toml`.
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

The detail waveform height is user-adjustable at runtime with `{` (decrease) and `}` (increase), defaulting to 8 rows. Any unused space below the panel is left blank.

The playhead column in the detail panel is set by `playhead_position` (0–100, default `20`) in the `[display]` section of `config.toml`. The value is a percentage of the panel width from the left edge; out-of-range values are clamped silently.

The detail waveform scrolls at half-column resolution: the viewport can be positioned at half-character offsets without modifying the pre-rendered buffer (see *Glossary — Half-column scrolling*).

The render frame period adapts to the current zoom level and detail panel width, targeting one dot-column advance per frame. At very tight zoom it is capped at ~120 fps; at very wide zoom it is capped at ~5 fps to keep input responsive.

### Needle Drop
- A left mouse click anywhere on the Overview waveform seeks the transport to the start of the nearest bar marker at or to the left of the click position. Playback state is preserved — if playing, playback continues from the new position; if paused, the transport remains paused. The Detail view recentres on the new position immediately.

### HPF / LPF Filter
- A single `filter_offset` parameter (range −16 to +16, default 0) controls a real-time second-order Butterworth IIR filter on the playback output:
  - `0` — flat (filter bypassed).
  - `−1` to `−16` — low-pass filter; more negative = lower cutoff frequency.
  - `+1` to `+16` — high-pass filter; more positive = higher cutoff frequency.
- `u` decreases `filter_offset` by 1 (clamped at −16); `7` increases it by 1 (clamped at +16).
- `Space+u` or `Space+7` snaps `filter_offset` to 0 (flat) immediately.
- Cutoff frequencies are logarithmically spaced from ~40 Hz to ~18 kHz across the ±1–±16 range. Each step corresponds to exactly one character of the spectrum strip.
- Filter state is visible in the spectrum strip (grey shading on attenuated bins) and not shown as separate text.
- The spectrum analyser reflects the filtered output.
- Filter state is not persisted between sessions; it always initialises to flat.

### Spectrum Analyser
- A compact spectrum analyser strip is displayed in the info bar, always active while a track is loaded. It is hidden during calibration mode.
- The strip is 16 braille characters wide (32 frequency bins) and 1 braille row tall (4 dot rows). Each character encodes two adjacent bins as a bottom-up bar chart. Thin `▕` / `▏` block characters flank the strip as bounds indicators. The bars are rendered in amber (yellow foreground on a dark amber background). When sub-threshold activity is detected in a bin (energy exceeds ¼ of the single-dot threshold), the character cell background is lit even if no dots are drawn, giving a background glow effect. The glow resets on a 2-bar accumulation window: it lights on any activity within the window and can only go dark at window boundaries.
- When a filter is active, the attenuated region of the spectrum is shaded with a grey background: LPF shades from the right, HPF from the left. Each of the 16 filter steps corresponds to exactly one spectrum character. At flat (offset 0) no shading is applied.
- Bins are logarithmically spaced from 20 Hz to 20 kHz. Amplitude is mapped on a dB scale (floor ~10 dB, ceiling ~60 dB, ~12.5 dB per dot row) using the Goertzel algorithm over a 4096-sample Hann-windowed window at the current playback position.
- The spectrum updates twice per beat period (every half beat). During BPM analysis the update interval falls back to 500 ms. The display holds its last value between updates.

### Audio Latency Calibration
- An `audio_latency_ms` value shifts all visual rendering backward by a fixed number of milliseconds, compensating for audio output latency. The effective display position is `smooth_display_samp − audio_latency_ms × sample_rate / 1000`. This affects the waveform viewport, beat markers, beat flash, and overview playhead.
- `~` toggles calibration mode. Calibration mode may only be entered while playback is paused. Pressing `~` again exits and persists the value immediately.
- While calibration mode is active (playback must be paused to enter):
  - The detail waveform and normal beat tick marks are hidden.
  - The playhead remains at its normal configured position.
  - A synthetic click tone fires at 60 BPM (every 1 second), injected directly into the mixer.
  - A calibration pulse marker (cyan, double-width tick) travels through the detail panel toward the playhead at the same 60 BPM tempo, arriving at the playhead at the moment the audio latency elapses after each click fires.
  - When a pulse marker coincides with the playhead, the playhead flashes bright red.
  - `d` / `c` adjust `audio_latency_ms` in 10ms steps (clamped 0–250ms). The value is snapped to the nearest 10ms on load and on calibration entry.
  - A vertical indicator line (`⣿`, U+28FF, dim steel blue `Rgb(80,100,140)`) shows the current `audio_latency_ms` position: playhead column = 0ms, right edge = 250ms.
  - Zoom in/out (`-`/`=`) is disabled while calibration mode is active. On entry the zoom resets to the default level (4s); on exit the previous zoom level is restored.
  - The info bar shows `lat:Nms  d/c adjust  ~ exit`; all other info bar content is hidden.
  - All other controls continue to function normally.
  - The user adjusts until the playhead flash coincides with the heard click.
- `audio_latency_ms` is stored as a single global value in the cache (alongside per-track entries). It is loaded on startup and saved on each change and on quit.
- The `[?]` help overlay lists `~` as the calibration key.

### Metronome
- `'` toggles metronome mode. While active, a click tone fires on every beat in sync with the current BPM and `offset_ms`. Only fires during playback; silent while paused or in calibration mode. No click fires on the beat coinciding with activation; clicks begin from the following beat.
- The metronome fires based on the audio buffer write position (ahead of the speaker by `audio_latency_ms`), so the click arrives at the speaker on the beat when latency is correctly calibrated.
- The click tone reuses the calibration click sound.
- A `♪` (U+266A) symbol in red is shown in the info bar immediately after the BPM value while metronome is active.
- Metronome mode resets to off on each new track load.

### Level Control
- The playback level is adjustable at runtime using `↑` (increase) and `↓` (decrease) in 5% steps, from 0% to 100%. The current level is displayed in the info bar as `level:N%`. Changes take effect immediately without interrupting playback. Level is not persisted between sessions.

### Nudge
- `c`/`d` nudge the transport backward/forward. Behaviour depends on the active nudge mode, toggled with `C`/`D`:
  - **`jump` mode** (default): each press (and key-repeat while held) seeks the playhead ±10ms.
  - **`warp` mode**: holding `c`/`d` applies a continuous ±10% speed offset; releasing returns to normal speed. While paused, drifts the transport position at ±10% of normal playback speed for as long as the key is held.
- The active nudge mode is shown in the info bar (`nudge:jump` / `nudge:warp`).
- While playing in warp mode, speed and pitch shift by ±10%; the audio output reflects the change within ~100ms.
- The nudge active state is indicated in the UI while a warp is held.
- While paused, each nudge step plays a short audio snippet at the new position — one half-column width of audio injected directly into the mixer. In jump mode a snippet fires on each key press/repeat; in warp mode snippets fire continuously at half-column intervals as the position drifts. Snippets play independently of the paused transport and do not interrupt each other.

### Beat Jump
- Eight dedicated beat jump actions cover four sizes (1, 4, 16, 64 beats) in each direction. Each action jumps by exactly N × beat_period seconds from the current position, preserving rhythmic continuity.
- Jumping backward past the start clamps to position 0. Jumping forward past the end is a no-op.
- Seeking is implemented via an atomic position counter shared with the audio thread; the audio thread never pauses.
- A ~6ms fade-out before the cut and ~6ms fade-in after eliminate click artefacts without any perceptible gap.

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

### Keyboard Mapping
- Key bindings are loaded from `config.toml` at startup — first from the same directory as the binary, then from `~/.config/tj/config.toml`. If neither file is found, the embedded default config is written to `~/.config/tj/config.toml` and loaded automatically.
- Bindings are declared under a `[keys]` table as `function_name = "key_string"` or `function_name = ["key1", "key2"]` for multiple keys per function.
- Key strings: printable characters as-is (`q`, `+`, `H`); special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`); `space+<key>` for Space-modifier chords (e.g. `space+z`).
- `Space` acts as a modifier key: holding it and pressing another key fires a chord action. `Space` released alone has no effect. The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.
- Ctrl-C always quits unconditionally and is not configurable.
- Display parameters are declared under a `[display]` table. Missing `[display]` keys fall back to their defaults; existing config files are never modified automatically.

## Constraints
- Implementation language: Rust.
- TUI framework: `ratatui`.
- Audio decoding: `symphonia`.
- Audio playback: `rodio`.
- BPM detection: `stratum-dsp`.
- Target platform: Linux (primary); other Unix-like systems are a stretch goal.

## Out of Scope (deferred)
- Cover art display.
- Shuffle, repeat.
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
