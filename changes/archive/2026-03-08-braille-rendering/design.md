# Design: Direct Braille Rendering
**Status: Ready for Review**

## Approach

Replace both Canvas widgets with `Paragraph` widgets whose content is pre-computed Braille characters. All dot rasterization moves off the UI thread. The render loop only assigns per-column colours and builds `Span` runs — O(cols) lightweight work per frame.

### Braille encoding

Unicode Braille (U+2800–U+28FF) encodes 8 dots per cell (2 wide × 4 tall):

```
dot1(bit0)  dot4(bit3)
dot2(bit1)  dot5(bit4)
dot3(bit2)  dot6(bit5)
dot7(bit6)  dot8(bit7)
```

A free function `render_braille(peaks: &[(f32,f32)], rows: usize, cols: usize) -> Vec<Vec<u8>>` produces a `rows × cols` grid of dot-pattern bytes. For each peak `(min, max)` at column `c`:
- Map `[−1, 1]` → `[0, rows×4)` dot rows (0 = top)
- Set both left and right dot columns (all 8 bits) for every dot row in `[top_dot, bot_dot]`

Empty cells (no peak range) remain `0x00` (rendered as U+2800, Braille blank — preserves column width).

### Colour assignment (UI thread, per frame)

For each waveform view, compute which column indices are "special":
- **Detail**: `beat_line_cols(bpm, offset_ms, pos_secs, zoom_secs, cols) -> Vec<usize>` (replaces `draw_beat_lines`); centre column = `cols / 2`
- **Overview**: `bar_tick_cols(bpm, offset_ms, total_secs, cols) -> Vec<usize>` (replaces `draw_bar_ticks`); playhead column = `(pos_frac * cols) as usize`

The UI thread then iterates columns, groups consecutive same-colour runs into `Span`s, and builds a `Line` per row:
- Beat/bar tick column → DarkGray (waveform char, or blank = marker visible in gaps)
- Centre / playhead column → White, char forced to `0xFF` (⣿, solid line through waveform)
- All other columns → waveform colour (Cyan for detail, Green for overview)

### Detail waveform thread

Extend the existing background thread:
- Add `detail_rows: Arc<AtomicUsize>` alongside `detail_cols`; UI writes it from `chunks[2].height` each frame
- Invalidation condition gains: `rows != last_rows`
- Output changes from `Arc<Vec<(f32,f32)>>` to `Arc<Vec<Vec<u8>>>` (dot grid); peaks are an intermediate, not stored
- `WaveformData::detail_peaks` is no longer called from the thread; the computation is inlined into a single pass that goes directly from samples to dot patterns

### Overview waveform

No dedicated thread needed — the overview peaks are already computed (`WaveformData.peaks`, 4000 buckets) and the Braille render is O(ov_cols × ov_rows) ≈ O(200 × 5) = very fast. The UI thread caches `last_ov_braille: Vec<Vec<u8>>` and re-renders it only when `ov_cols` or `ov_rows` differs from the last render (i.e. on resize). Playhead and bar tick columns are overlaid each frame via colour assignment.

### Paragraph layout

Both waveform views change from `Canvas::default()...` to `Paragraph::new(lines)` where `lines: Vec<Line<'static>>`. No `Block` border is needed (existing border comes from the outer block). Layout constraints stay the same.

### Functions removed / replaced

| Before | After |
|---|---|
| `draw_bar_ticks` (Canvas draw fn) | `bar_tick_cols` (returns `Vec<usize>`) |
| `draw_beat_lines` (Canvas draw fn) | `beat_line_cols` (returns `Vec<usize>`) |
| `WaveformData::detail_peaks` | `render_braille` (free fn, used by bg thread) |

`WaveformData::detail_peaks` is deleted; `render_braille` handles the full sample→Braille path.

## Tasks

1. ✓ Impl: Add `render_braille(peaks, rows, cols) -> Vec<Vec<u8>>` free function; add `beat_line_cols` and `bar_tick_cols` free functions. (Deletions of `draw_bar_ticks`, `draw_beat_lines`, `WaveformData::detail_peaks` deferred to Tasks 2–4 as each caller is replaced.)
2. ✓ Impl: Extend detail background thread — add `detail_rows` atomic, change output to `Arc<Vec<Vec<u8>>>`, inline sample→Braille computation.
3. ✓ Impl: Replace detail Canvas with Paragraph — read dot grid Arc, compute beat cols, build coloured Spans.
4. ✓ Impl: Replace overview Canvas with Braille — render fresh each frame (O(cols×rows), negligible cost), overlay playhead and bar tick colours each frame.
5. Verify: `cargo build`; manual test — both views render correctly, beat markers visible, resize works, no stutter.
6. Process: Confirm ready to archive.
