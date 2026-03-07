# Design: Waveform Visualisation
**Status: Approved**

## Approach

### Rendering
Use ratatui's `Canvas` widget with `Marker::Braille` (2×4 sub-character resolution). For each display column, draw a vertical `Line` from the minimum to maximum amplitude in the corresponding audio chunk. This gives a classic symmetric waveform display centred on zero.

Braille gives 4× vertical resolution over block characters, making the waveform readable even at small heights.

### Waveform Data Precomputation
On load, after decode, reduce the full PCM array to a fixed-resolution peak table:
- Divide the mono samples into `OVERVIEW_RESOLUTION = 4000` equal chunks.
- For each chunk store `(min: f32, max: f32)` — the amplitude envelope.
- This is computed once; rendering maps from pixel columns to this table at draw time.

The detail view computes its envelope on the fly from the raw samples for the visible window.

### Overview View
- Spans the full track width.
- Canvas `x_bounds = [0.0, width]`, `y_bounds = [-1.0, 1.0]`.
- Each column `x` maps to the corresponding entry in the peak table.
- Playhead: a white vertical line at the column corresponding to current playback position.

### Detail View
- Shows a configurable window of audio centred on the playhead.
- Default zoom: 4 seconds visible. Adjustable with `z` (zoom in) and `Z` (zoom out).
- Zoom levels: 1s, 2s, 4s, 8s, 16s, 32s (doubles/halves each step).
- Computes envelope from raw samples for the visible window at render time.
- Playhead always at the horizontal centre.

### TUI Layout
```
┌─ tj — filename ──────────────────────────┐
│ BPM: 120.00   offset: +0ms               │  <- 1 line
├──────────────────────────────────────────┤
│                                          │
│  [Overview waveform + playhead]          │  <- ~5 lines
│                                          │
├──────────────────────────────────────────┤
│                                          │
│  [Detail waveform + playhead]            │  <- ~8 lines
│                                          │
├──────────────────────────────────────────┤
│  ██  BEAT  ██                            │  <- 1 line
│  [Playing]  03:21 / 07:45               │  <- 1 line
│  Space …  +/-: offset  z/Z: zoom  q     │  <- 1 line
└──────────────────────────────────────────┘
```

### Performance
- Overview peak table is precomputed — rendering is O(width), no audio data touched.
- Detail envelope computed per frame from a small slice of samples — fast enough at 30fps.
- No additional threads needed; all rendering on the main thread.

## Tasks
1. ✓ **Impl**: Precompute `WaveformData` peak table from mono samples after decode.
2. ✓ **Impl**: Add overview `Canvas` (Braille, green) to TUI layout with white playhead line.
3. ✓ **Impl**: Add detail `Canvas` (Braille, cyan) with zoom state; `z`/`Z` keys; 6 zoom levels (1–32s).
4. ✓ **Impl**: Updated key hints and status line (shows current zoom).
5. ✓ **Verify**: Overview renders full track on launch; playhead tracks in real time; detail zoom works.
6. ✓ **Process**: Confirm ready to archive.
