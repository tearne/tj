# Album Art ASCII
**Type**: Spike
**Status**: Active

## Goal
Explore whether small ASCII/braille art representations of embedded album art are feasible and worth adding to the UI. Target placement: the black space below the decks, as a low-risk visual experiment.

## Log

### 2026-03-26 — Initial research

**Cover art extraction (lofty 0.22)**

Already a dependency. `lofty::read_from_path(path)` → `tagged_file.primary_tag()` → `tag.pictures()` returns `&[Picture]`. `picture.data()` gives raw image bytes; `picture.mime_type()` returns JPEG/PNG/etc. Can target `PictureType::CoverFront` specifically. No new dependency needed for extraction — just the bytes.

**Image-to-terminal rendering options**

Four realistic options identified:

| Crate | Approach | `image` dep needed | Ratatui fit |
|---|---|---|---|
| `ratatui-image` | Sixel/Kitty/half-block widget | yes | native widget |
| `viuer` | half-blocks (`▄`), Sixel/Kitty | yes | stdout-oriented |
| `rascii_art` | ASCII chars, ANSI colour | yes | string → Paragraph |
| manual | `image` crate downsample → block/braille chars | yes | full control |

All require adding the `image` crate for JPEG/PNG decoding.

**Key open questions**

1. How much vertical space is actually available below the decks in the current layout? Need to audit `src/render/mod.rs`.
2. Does the terminal running tj support Sixel or Kitty? If not, half-blocks are the fallback.
3. `ratatui-image` is the cleanest fit (native widget, auto protocol detection) but adds a non-trivial dependency. A manual approach with `image` + half-block characters keeps the dependency surface smaller and is simpler to remove if the feature is dropped.
4. Should art be shown for both decks, or only the active/playing deck?

**Placement decision: below the decks**

Two options were considered:

- **Below the decks** — the main layout (`src/main.rs:544`) already has a `rem` spacer slot (`c[11]`) that consumes leftover vertical space. No layout change needed; just render into it. Art updates when a track loads. Will simply not appear if the terminal is too short.
- **Browser RHS** — the browser is currently a single `Min(0)` list + 1-row status bar. Would require a new horizontal split and art that re-reads on every cursor move. More complex.

**Decision**: below the decks. Use a fixed render height of ~10 rows (≈20 pixels with half-blocks); if the spacer is smaller it just won't show. Two art panels side by side — one per deck — using the full width of the spacer split 50/50.

**POC plan**

1. Add `image` crate (decode + resize only — no extra features).
2. New function `read_cover_art(path) -> Option<Vec<u8>>` in `src/tags/mod.rs` via lofty `tag.pictures()`, targeting `PictureType::CoverFront` first, falling back to first picture.
3. New function `halfblock_art(bytes: &[u8], cols: u16, rows: u16) -> Vec<Line<'static>>` — decode with `image`, resize to `cols × (rows*2)`, map each vertically-adjacent pixel pair to `▀` (fg=top, bg=bottom colour).
4. Cache the rendered `Vec<Line>` in the deck state when a track loads (re-render only on load).
5. In the render loop, split `c[11]` horizontally 50/50 and render each deck's art panel.

## Outcome

### 2026-03-27 — POC implemented

POC is complete and building clean. Implementation matches the plan from initial research:

- `image` crate added (JPEG + PNG decode, no extra features)
- `read_cover_art(path)` in `src/tags/mod.rs` — extracts `CoverFront` picture via lofty, falling back to first picture
- `halfblock_art(bytes, cols, rows)` in `src/render/mod.rs` — decodes, resizes to `cols × (rows*2)`, maps pixel pairs to `▀` with true-colour fg/bg
- `cover_art: Option<Vec<u8>>` on `Deck` — set in `build_deck` after decode
- Spacer row `c[11]` split 50/50 horizontally — each deck's art rendered if present and area ≥ 2 rows tall

Ready for live testing. Key questions to resolve:
1. Does art render correctly on the host terminal?
2. Is `image::load_from_memory` + resize-per-frame too slow at 50ms frame times, or fast enough?
3. Is 50/50 side-by-side the right layout, or should art only show for the loaded deck?

### 2026-03-27 — Refinements

Following live testing:

- Switched to fill (cover) mode: image scales to cover the full panel, cropping the shorter axis symmetrically — no letterboxing
- Added 1-row top margin and 1-column centre gap (black border separating art from the deck waveforms above and from each other)
- Added `\` art cycle key: full → dim (35%) → off → full, via `art_state: u8` in the tui loop and a `brightness: f32` parameter on `halfblock_art`
