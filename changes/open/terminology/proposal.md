# Proposal: Rendering Terminology
**Status: Draft**

## Intent
Define a shared vocabulary for the rendering pipeline so that the specification can express implementation requirements precisely, without resorting to inline explanation. Clear terminology also helps a future implementer avoid the pitfalls we have already diagnosed and resolved.

---

## Candidate Terms

### The rendering pipeline

Waveform rendering proceeds in two stages:

```
Audio samples ──[background thread]──▶ Braille buffer ──[UI thread]──▶ Screen
  (sample space)                        (buffer space)               (screen space)
```

The background thread rasterises peaks into a buffer wider than the screen. The UI thread slides a viewport through the buffer each frame, applying a half-column shift when needed, and passes the result to the terminal.

---

**Sample space**
The coordinate of raw audio data. A position is an integer sample index from 0 (start of track) to `total_samples − 1`.

**Column grid**
A coordinate system that partitions the timeline into discrete character-column cells, each `samples_per_col` samples wide. Cell `n` spans `[n × samples_per_col, (n+1) × samples_per_col)`. The grid is unbounded — cells extend before sample 0, which is what allows pre-track cells to render as silence rather than clamping to the first sample. Any two buffers computed at the same zoom level share identical cell boundaries wherever they overlap, making overlapping cells byte-for-byte equal.

**Buffer space**
The coordinate of the pre-rendered braille byte buffer: an array of cells indexed 0 to `buf_cols − 1`, each corresponding to one column-grid cell. The buffer is wider than the screen and centred on the **anchor** — the column-grid cell nearest the current playhead position. Elements that must appear in screen space (such as beat tick marks) should not be computed in buffer space: the half-column shift transforms isolated marks into different braille characters on alternating frames, causing visible oscillation.

**Screen space**
The coordinate of visible screen columns, indexed 0 (left edge) to `dw − 1` (right edge). The playhead is fixed at `centre_col`. At half-column resolution, positions are expressed in half-character units — even values are the left half of a character, odd values the right half — so that tick marks can be placed between character boundaries.

---

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

---

### Rendering positions

**Smooth display position**
The sample position used as the rendering playhead. It advances by wall-clock elapsed time rather than from the audio output position (which advances in bursts). After a large drift — on seek or startup — it snaps to the nearest column-grid boundary, ensuring `sub_col = false` immediately after a seek. The single source of truth for all rendering.

**Quantised viewport centre**
The smooth display position rounded to the nearest half-column boundary. Both the waveform viewport and beat tick marks must be derived from this value — not from the raw smooth display position, which can differ by up to half a column, causing visible oscillation at wide zoom.

---

## Specification Deltas

### ADDED
- `## Glossary` section in `SPEC.md` containing the terms above.
- References to these terms throughout the existing Rendering section where they aid clarity.
