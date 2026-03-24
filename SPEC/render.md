# Render

## UI Layout

### Structure

The UI is structured into the following vertical sections (top to bottom):
1. Detail info bar (shared)
2. Detail waveform — Deck A
3. Detail waveform — Deck B
4. Notification bar — Deck A
5. Info bar — Deck A
6. Overview — Deck A
7. Notification bar — Deck B
8. Info bar — Deck B
9. Overview — Deck B
10. Global status bar

### Global Status Bar

A single row pinned to the bottom of the UI. Content priority:
1. **System notification** — transient messages not tied to either deck (e.g. config parse warning, startup prompt). Shown until expired; uses the same `Notification` type and expiry mechanism as per-deck notifications.
2. **Idle status** — shown when no system notification is active. Displays the current browser working directory in dim style.

### Notification Bar

- A single line displayed above the info bar. By default it shows the track name derived from embedded metadata: `Artist – Title` if both are present, `Title` if only a title is available, or the filename as a fallback. Shown only when a track is loaded.
- When a notification is active it temporarily replaces the track name. Notifications carry a message, a style (`Info` / `Warning` / `Error`), and an expiry; the most recent notification takes precedence. Notifications expire automatically after their timeout; no explicit dismissal is required.
- The track name is rendered in a muted form of the active palette's treble colour, distinguishing it visually from notification text.
- The BPM confirmation prompt (see `deck/SPEC.md`) is displayed as a `Warning`-style notification.
- If no config file is found on first launch, an `Info` notification briefly displays the path at which the default config was created, then the bar reverts to the track name.

### Info Bar

- A single line below the track name bar. Content is split into two groups separated by a variable-width spacer that fills remaining width, keeping the right group pinned to the right edge regardless of transient field changes:
  - **Left group**: play/pause icon (`▶`/`⏸`), BPM, `♪` in red when metronome is active, phase offset. Tap count (`tap:N`) appended transiently while a tap session is active.
  - **Right group**: nudge mode (`nudge:jump` / `nudge:warp`, fixed width), level (`level:▕N▏` — single eighth-block character in dark yellow, in a bracketed indicator with mid-grey brackets), `lat:Xms` (shown only when `audio_latency_ms > 0`), spectrum strip.
- The nudge mode field is always present and fixed-width so toggling between `jump` and `warp` does not shift anything to its right.
- When no tempo adjustment is active, the detected BPM is shown to two decimal places (e.g. `120.00`) and receives a soft amber beat-flash. When a per-deck BPM adjustment is active, the detected BPM is shown plain and the adjusted tempo is shown alongside in parentheses (e.g. `120.00 (124.40)`), with only the adjusted number receiving the beat-flash.
- Pressing `?` opens a modal key binding reference overlay; any key dismisses it.
- During BPM analysis the BPM field shows an animated spinner. When a confirmation is pending, the prompt appears in the notification bar; the right group is always rendered normally.
- A BPM is considered "established" once it has been loaded from cache, set by tap, or adjusted via the per-deck BPM keys. Only established BPM triggers confirmation on new detection.

### Empty Deck Panels

When no track is loaded in a deck slot, all deck sections render at full height with placeholder content:
- **Notification bar**: dim deck label ("A" or "B") and prompt "no track — press z to open the file browser".
- **Info bar**: `⏸  ---  +0ms` in dim style; level and filter widgets omitted.
- **Overview**: a faint flat horizontal line at the vertical midpoint, rendered via the braille pipeline with zero-amplitude peaks and 120 BPM tick marks.
- **Detail waveform**: a faint vertical line at the playhead column spanning the full height; all other columns blank.

Layout constraints are based on the loaded deck's `detail_height` (defaulting to 8 rows), so no section collapses to zero when a deck slot is empty.

---

## Waveforms

### Overview Waveform

- Displays the full-track waveform with a playhead marker showing current position.
- Updates in real time during playback.
- Coloured by spectral content: each column blends between an orange/warm colour (bass-heavy) and a cyan/cool colour (treble-heavy), based on the ratio of low-frequency to high-frequency energy in that section. The frequency crossover is ~250 Hz. Several colour palettes are available and cycle with `p`; the active palette name is shown in the info bar.
- Rendered at half-column braille resolution: each braille character encodes two independent audio columns (left dot column and right dot column), doubling the horizontal detail within the same screen width.
- Displays bar markers as thin vertical lines (`│`) spanning the full overview height. The marker interval starts at every 4 bars and doubles if there are fewer than four characters between any pair of adjacent markers, repeating until all pairs have at least four characters between them. A legend in the top-right corner of the Overview shows the current interval (e.g. `4 bars`, `8 bars`).
- When the remaining playback time falls below a configurable threshold (default 30 seconds), the overview bar markers flash in time with the BPM: the marker colour alternates each beat (one beat on, one beat off) using a muted reddish-grey. The warning is active only during playback. The threshold is configurable via `warning_threshold_secs` in the `[display]` section of `config.toml`.

### Detail Waveform

- Displays a zoomed waveform centred on the playhead, with variable zoom level.
- Updates in real time during playback; tracks the playhead as the track progresses.
- Displays a beat marker at each beat position as a full-height line drawn beneath the waveform, visible only in the gaps.
- The column grid is scaled by each deck's `bpm / base_bpm` ratio, so the viewport is expressed in playback-time columns. Beat tick marks placed at `base_bpm` sample spacing therefore appear at `bpm`-spaced columns: two decks at the same effective BPM show identical, waveform-anchored tick grids.
- Buffer columns representing sample positions before the start of the track render as silence (zero amplitude), not as a mirror of position 0.

### Shared Behaviour

- Zoom level and detail height are shared across both decks. `-`/`=` and `{`/`}` adjust them globally.
- Both sets of markers shift immediately when the phase offset is adjusted.
- The detail info bar is a single shared row above both detail waveforms, showing the common zoom level (e.g. `zoom:4s`) in dim style.
- The detail waveform height is user-adjustable at runtime with `{` (decrease) and `}` (increase), and applies to both decks simultaneously. Any unused space below the panel is left blank. The initial height is set by `detail_height` in the `[display]` section of `config.toml` (default `6`, minimum `3`; value is total rows including the 2-row tick area, giving 4 waveform rows at the default).
- The playhead column in the detail panel is set by `playhead_position` (0–100, default `20`) in the `[display]` section of `config.toml`. The value is a percentage of the panel width from the left edge; out-of-range values are clamped silently.
- The detail waveform scrolls at half-column resolution: the viewport can be positioned at half-character offsets without modifying the pre-rendered buffer (see *Glossary — Half-column scrolling*).

### Spectrum Analyser

- A compact spectrum analyser strip is displayed in the info bar, always active while a track is loaded.
- The strip is 16 braille characters wide (32 frequency bins) and 1 braille row tall (4 dot rows). Each character encodes two adjacent bins as a bottom-up bar chart. Thin `▕` / `▏` block characters flank the strip as bounds indicators. The bars are rendered in amber (yellow foreground on a dark amber background). When sub-threshold activity is detected in a bin (energy exceeds ¼ of the single-dot threshold), the character cell background is lit even if no dots are drawn, giving a background glow effect. The glow resets on a 2-bar accumulation window: it lights on any activity within the window and can only go dark at window boundaries.
- When a filter is active, the attenuated region of the spectrum is shaded with a grey background: LPF shades from the right, HPF from the left. Each of the 16 filter steps corresponds to exactly one spectrum character. At flat (offset 0) no shading is applied.
- Bins are logarithmically spaced from 20 Hz to 20 kHz. Amplitude is mapped on a dB scale (floor ~10 dB, ceiling ~60 dB, ~12.5 dB per dot row) using the Goertzel algorithm over a 4096-sample Hann-windowed window at the current playback position.
- The spectrum updates twice per beat period (every half beat). During BPM analysis the update interval falls back to 500 ms. The display holds its last value between updates. Both decks' spectra update at this cadence regardless of which deck is active.

---

## Rendering Pipeline

### Overview

Waveform rendering proceeds in two stages:

```
Audio samples ──[background thread]──▶ Braille buffer ──[UI thread]──▶ Screen
  (sample space)                        (buffer space)               (screen space)
```

The background thread rasterises peaks into a buffer wider than the screen. The UI thread slides a viewport through the buffer each frame, applying a half-column shift when needed, and passes the result to the terminal.

### Coordinate Spaces

**Sample space** — The coordinate of raw audio data. A position is an integer sample index from 0 (start of track) to `total_samples − 1`.

**Column grid** — A coordinate system that partitions the timeline into discrete character-column cells, each `samples_per_col` samples wide. Cell `n` spans `[n × samples_per_col, (n+1) × samples_per_col)`. The grid is unbounded — cells extend before sample 0, which is what allows pre-track cells to render as silence rather than clamping to the first sample. Any two buffers computed at the same zoom level share identical cell boundaries wherever they overlap, making overlapping cells byte-for-byte equal.

**Buffer space** — The coordinate of the pre-rendered braille byte buffer: an array of cells indexed 0 to `buf_cols − 1`, each corresponding to one column-grid cell. The buffer is wider than the screen and centred on the **anchor** — the column-grid cell nearest the current playhead position. Each buffer holds three parallel arrays:
- `grid`: `rows × buf_cols` braille bytes — the waveform dot patterns
- `tick`: `buf_cols` semantic tick values (see *Tick Encoding* below)
- `cue_buf_col`: a single `Option<usize>` — the buffer column of the cue point, if set and in range

**Screen space** — The coordinate of visible screen columns, indexed 0 (left edge) to `dw − 1` (right edge). The playhead is fixed at `centre_col`. At half-column resolution, positions are expressed in half-character units — even values are the left half of a character, odd values the right half — so that tick marks can be placed between character boundaries.

### Background Thread Guarantees

The following invariants must be maintained to achieve smooth, stable rendering:

- **Smooth display position**: The position used for all visual rendering (detail viewport, beat markers) advances by wall-clock elapsed time rather than reading the audio output position directly. The audio output position advances in bursts as the output device requests audio buffers; using it directly causes visible periodic jumps. Small accumulated drift (>20 ms) is corrected gradually. Large drift (e.g. after seek or startup) is snapped to the nearest **column-grid** boundary — not to the raw sample position — ensuring `sub_col = false` immediately after every seek.
- **Consistent position**: Beat marker columns must be computed from the same **smooth display position** as the waveform viewport. Using different position sources causes markers to oscillate relative to the waveform.
- **Waveform computation off the UI thread**: Braille dot rasterisation runs on a background thread, producing a buffer in **buffer space**. The UI thread performs only lightweight per-frame work (translating to **screen space**, colour assignment, span construction) to stay within the frame budget.
- **Stable buffer between recomputes**: The background thread pre-renders a buffer wider than the visible area. The UI thread slides a viewport through it each frame, avoiding a full recompute on every tick. Passing an unchanged buffer to ratatui prevents a full widget repaint and visible flicker.
- **Background thread uses smooth position**: The background thread must use the **smooth display position** as both the drift trigger and the buffer centre. Using the raw audio position causes premature recomputes on audio bursts and centres the buffer at a different position than the viewport expects, producing a visible jump on each recompute.
- **Fixed column grid**: The **anchor** is aligned to the **column grid** and peaks are computed by direct per-column indexing. This ensures any two buffers at the same zoom level share identical cell boundaries, so overlapping cells are byte-for-byte equal and the buffer handoff is visually seamless.
- **Early recompute trigger**: The background thread begins computing a new buffer when drift reaches 3/4 of the screen width (not at the edge), ensuring the new buffer is ready before the old one runs out. A last-valid-viewport fallback prevents black frames in the rare case the OS delays the background thread.
- **Overlay markers in the buffer pipeline**: Beat tick marks and the cue point line are computed by the background thread at the same time as the waveform, stored in the buffer using the same anchor and `samples_per_col`. The draw thread maps them to screen coordinates via the same `viewport_start` transform as the waveform, guaranteeing they cannot drift relative to it.

Both detail waveforms are rendered by a single shared background thread in the same pass at identical `samples_per_col`, ensuring their column grids are byte-for-byte compatible and both viewports advance at the same rate each frame.

The render frame period adapts to the current zoom level and detail panel width, targeting one dot-column advance per frame. At very tight zoom it is capped at ~120 fps; at very wide zoom it is capped at ~5 fps to keep input responsive.

### Half-Column Scrolling

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

### Tick Encoding

Beat tick marks are stored in the `tick` array as **semantic sub-column values**, not as raw braille bit patterns. This is essential: storing raw braille would cause the half-column shift (`shift_braille_half`) to scramble the tick marks, since the shift combines bits from adjacent cells and the result would place dot fragments in the wrong columns.

The two sentinel values and their derivation:

| Value | Meaning | Derivation |
|-------|---------|-----------|
| `0x47` | Tick on left sub-column | Braille dots 1,2,3,7 (the left dot column, all four rows lit, plus dot 7 for the lower-half extension) — encodes a full-height vertical bar in the **left** half of the character cell |
| `0xB8` | Tick on right sub-column | Braille dots 4,5,6,8 (the right dot column, all four rows lit, plus dot 8) — encodes a full-height vertical bar in the **right** half of the character cell |
| `0x00` | No tick | Empty |

The braille encoding maps dots to bits as follows (Unicode braille block U+2800):

```
Dot layout:    Bit positions:
  1  4           bit 0  bit 3
  2  5           bit 1  bit 4
  3  6           bit 2  bit 5
  7  8           bit 6  bit 7
```

So `0x47` = bits 0,1,2,6 = dots 1,2,3,7 (left column, full height). `0xB8` = bits 3,4,5,7 = dots 4,5,6,8 (right column, full height).

**Half-column shift transform for ticks**: When `sub_col=true`, the draw thread extracts the tick value and shifts it: a `0x47` tick (left sub-column) on cell `n` becomes a `0xB8` tick (right sub-column) because the left dot-column of cell `n+1` is displayed in the right half of screen column `c`. A `0xB8` tick on cell `n` becomes a `0x47` tick for screen column `c` (it was on the right half, now it's the left half of a shifted cell). This transform is applied explicitly by `extract_tick_viewport` — it is not an automatic consequence of `shift_braille_half`.

### Cue Mark

The cue point is stored in `BrailleBuffer` as `cue_buf_col: Option<usize>` — a single buffer column index. This differs from tick marks in two ways: the cue spans the full character width (not a sub-column), and it does not need a half-column shift transform because it is mapped to the nearest screen column by integer division.

**Rendering**: The draw thread maps `cue_buf_col` to a screen column via `viewport_start`: `screen_col = cue_buf_col − viewport_start`. If the result is within `[0, screen_width)`, the cue is rendered as a green `│` character spanning the full detail height, replacing whatever waveform or tick content would otherwise appear at that column.

### Column Coincidence Rules

When multiple elements compete for the same screen column, the draw thread applies this priority order (highest wins):

1. **Playhead** — always rendered; drawn as a distinct playhead marker (vertical line in the playhead colour), overrides everything
2. **Cue mark** — green `│`, overrides waveform and tick marks
3. **Tick mark** — rendered beneath waveform content in the gaps where waveform dots are absent; does not override waveform dots (only visible in empty rows)
4. **Waveform** — the braille dot pattern from `BrailleBuffer.grid`

The cue and tick marks are rendered in the gaps between waveform dots rather than as overlays, which is why the waveform is always computed first and the markers are applied per-row by checking for empty dot positions.

### Rendering Positions

**Smooth display position** — The sample position used as the rendering playhead. It advances by wall-clock elapsed time rather than from the audio output position (which advances in bursts). After a large drift — on seek or startup — it snaps to the nearest column-grid boundary, ensuring `sub_col = false` immediately after a seek. The single source of truth for all rendering.

**Quantised viewport centre** — The smooth display position rounded to the nearest half-column boundary. The waveform viewport is derived from this value. Because tick marks are rendered in the same background pass as the waveform and share the same anchor, they are automatically aligned — no separate quantisation is required at draw time.
