# Waveforms

## Overview Waveform

- Displays the full-track waveform with a playhead marker showing current position.
- Updates in real time during playback.
- Coloured by spectral content: each column blends between an orange/warm colour (bass-heavy) and a cyan/cool colour (treble-heavy), based on the ratio of low-frequency to high-frequency energy in that section. The frequency crossover is ~250 Hz. Several colour palettes are available and cycle with `p`; the active palette name is shown in the info bar.
- Rendered at half-column braille resolution: each braille character encodes two independent audio columns (left dot column and right dot column), doubling the horizontal detail within the same screen width.
- Displays bar markers as thin vertical lines (`│`) spanning the full overview height. The marker interval starts at every 4 bars and doubles if there are fewer than four characters between any pair of adjacent markers, repeating until all pairs have at least four characters between them. A legend in the top-right corner of the Overview shows the current interval (e.g. `4 bars`, `8 bars`).
- When the remaining playback time falls below a configurable threshold (default 30 seconds), the overview bar markers flash in time with the BPM: the marker colour alternates each beat (one beat on, one beat off) using a muted reddish-grey. The warning is active only during playback. The threshold is configurable via `warning_threshold_secs` in the `[display]` section of `config.toml`.

## Detail Waveform

- Displays a zoomed waveform centred on the playhead, with variable zoom level.
- Updates in real time during playback; tracks the playhead as the track progresses.
- Displays a beat marker at each beat position as a full-height line drawn beneath the waveform, visible only in the gaps.
- The column grid is scaled by each deck's `bpm / base_bpm` ratio, so the viewport is expressed in playback-time columns. Beat tick marks placed at `base_bpm` sample spacing therefore appear at `bpm`-spaced columns: two decks at the same effective BPM show identical, waveform-anchored tick grids.
- Buffer columns representing sample positions before the start of the track render as silence (zero amplitude), not as a mirror of position 0.

## Shared Behaviour

- Zoom level and detail height are shared across both decks. `-`/`=` and `{`/`}` adjust them globally.
- Both sets of markers shift immediately when the phase offset is adjusted.
- The detail info bar is a single shared row above both detail waveforms, showing the common zoom level (e.g. `zoom:4s`) in dim style.
- The detail waveform height is user-adjustable at runtime with `{` (decrease) and `}` (increase), and applies to both decks simultaneously. Any unused space below the panel is left blank. The initial height is set by `detail_height` in the `[display]` section of `config.toml` (default `6`, minimum `3`; value is total rows including the 2-row tick area, giving 4 waveform rows at the default).
- The playhead column in the detail panel is set by `playhead_position` (0–100, default `20`) in the `[display]` section of `config.toml`. The value is a percentage of the panel width from the left edge; out-of-range values are clamped silently.
- The detail waveform scrolls at half-column resolution: the viewport can be positioned at half-character offsets without modifying the pre-rendered buffer (see *Glossary — Half-column scrolling*).

## Spectrum Analyser

- A compact spectrum analyser strip is displayed in the info bar, always active while a track is loaded.
- The strip is 16 braille characters wide (32 frequency bins) and 1 braille row tall (4 dot rows). Each character encodes two adjacent bins as a bottom-up bar chart. Thin `▕` / `▏` block characters flank the strip as bounds indicators. The bars are rendered in amber (yellow foreground on a dark amber background). When sub-threshold activity is detected in a bin (energy exceeds ¼ of the single-dot threshold), the character cell background is lit even if no dots are drawn, giving a background glow effect. The glow resets on a 2-bar accumulation window: it lights on any activity within the window and can only go dark at window boundaries.
- When a filter is active, the attenuated region of the spectrum is shaded with a grey background: LPF shades from the right, HPF from the left. Each of the 16 filter steps corresponds to exactly one spectrum character. At flat (offset 0) no shading is applied.
- Bins are logarithmically spaced from 20 Hz to 20 kHz. Amplitude is mapped on a dB scale (floor ~10 dB, ceiling ~60 dB, ~12.5 dB per dot row) using the Goertzel algorithm over a 4096-sample Hann-windowed window at the current playback position.
- The spectrum updates twice per beat period (every half beat). During BPM analysis the update interval falls back to 500 ms. The display holds its last value between updates. Both decks' spectra update at this cadence regardless of which deck is active.

## Rendering

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

Both detail waveforms are rendered by a single shared background thread in the same pass at identical `samples_per_col`, ensuring their column grids are byte-for-byte compatible and both viewports advance at the same rate each frame.

The render frame period adapts to the current zoom level and detail panel width, targeting one dot-column advance per frame. At very tight zoom it is capped at ~120 fps; at very wide zoom it is capped at ~5 fps to keep input responsive.

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
