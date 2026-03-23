# Waveform Palettes — Findings

## Q1: Does a multi-stop gradient read better than two-stop?

Yes. Four stops give enough range to make spectral content feel expressive and legible at typical terminal sizes. Three stops were implemented but extended to four during the spike; the extra segment adds meaningful visual resolution in the mid-bass/mid-treble range without feeling busy.

## Q2: What pairing schemes work well?

Only one scheme was kept from testing: **amber/cyan** (both decks identical). The other three experiments (warm-rainbow, sunset, ember) were less compelling. Per-deck contrast via hue pairing was not pursued — the schemes tested used the same palette on both decks.

## Q3: Palette data structure

`SpecPalette` was extended from 3 stops to 4: `(treble, mid-treble, mid-bass, bass)`. Interpolation maps `bass_ratio ∈ [0,1]` to `t = bass * 3`, selects a segment by floor, and lerps within it. Normalisation (`255 / max_channel`) is applied after interpolation to preserve full saturation at every blend point.

## Q4: Pairing — automatic or manual?

Manual. A single `P` key cycles a global `scheme_idx` that sets both decks' palettes simultaneously from a `PALETTE_SCHEMES` table. This is simple and keeps user control explicit.

## Additional outcomes

- **IIR low-pass bass detection**: `diff_energy` heuristic (which was phase-delayed) replaced with a per-column IIR filter at 250 Hz applied during `WaveformData::compute`. Bass colour now arrives on the beat rather than slightly after it.
- **Box smoothing**: `box_smooth(radius=3)` applied to `bass_ratio` after the IIR pass to prevent sharp colour transitions at wide zoom levels.
- **Amplitude dimming removed**: brightness floor experiments (various levels) were ultimately removed. All waveform columns render at full saturation.
- **Overview brightness**: overview waveform dimmed to 0.8× to visually subordinate it to the detail view.
- **Marker colours**: playhead uses `Rgb(255,255,255)` (true white), cue uses `Rgb(255,0,255)` (magenta) — both at full brightness, unaffected by the overview dimming or terminal named-colour interpretation. Green washed out against the amber/cyan palette; magenta is clearly distinct.
- **Buffer sizing bug fixed**: the shared braille buffer was sized to `h - 1` rows (to leave room for deck A's shared tick row). Deck B's waveform uses the full `h` rows, so its last row fell off the buffer and rendered blank — making the playhead/cue overlap look different between decks. Fixed by sizing the buffer to `h` rows.
