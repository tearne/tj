# Proposal: Keymap Redesign
**Status: Approved**

## Goal

Redesign the keyboard layout so that both decks are simultaneously accessible without requiring a deck-select key (`g`/`h`). On application load, two decks exist but are empty (zero waveform). The user loads tracks directly into each deck via the browser without switching context.

The current layout was designed for a single active deck and retrofitted for two; the new layout should treat two-deck operation as the primary model.

## Modifier Convention

- **Space** — common modifiers (actions used during a set: play/pause, cue, beat jumps, level snap, filter reset)
- **Shift** — rare or configuration modifiers (base BPM, browser load, metronome, nudge mode, waveform/palette)

Using only these two modifiers keeps the layout reachable without leaving home position and avoids terminal compatibility issues from complex modifier combinations.

## Current Keymap

See [`current-keymap.md`](./current-keymap.md) for the full baseline.

## Design Decisions

| # | Decision | Resolution |
|---|----------|------------|
| 1 | Load into Deck 1 / Deck 2 | `Shift+Z` / `Shift+C` |
| 2 | Level controls | `j`/`m` (D1), `k`/`,` (D2); `Space+j`/`Space+m` snap max/min (D1), `Space+k`/`Space+,` snap max/min (D2) |
| 3 | Filter controls | `7`=D1 cut bass, `u`=D1 cut treble, `i`=D2 cut bass, `8`=D2 cut treble; `Space+<filter key>` resets that deck's filter |
| 4 | Jumps | Per-deck; `1`/`2`/`q`/`w` (D1), `3`/`4`/`e`/`r` (D2); plain=bars, Space=beats |
| 5 | Play/pause | Per-deck: `Space+Z` (D1), `Space+C` (D2) |
| 6 | Cue | Per-deck: `Space+A` (D1), `Space+D` (D2); CDJ-style hold-to-play via separate proposal |
| 7 | BPM tap | Per-deck: `b` (D1), `n` (D2) |
| 8 | Metronome | Per-deck: `Shift+B` (D1), `Shift+N` (D2) |
| 9 | BPM redetect | Per-deck: `'` (D1), `#` (D2) |
| 10 | Tempo reset | Tentative: `Shift+'` (D1), `Shift+#` (D2) |
| 11 | Playback BPM | Per-deck pitch-slider convention (up=slow, down=fast): `s`/`x` (D1), `f`/`v` (D2), ±0.1 BPM |
| 12 | Base BPM | Per-deck: `Shift+S`/`Shift+X` (D1), `Shift+F`/`Shift+V` (D2), ±0.01 BPM |
| 13 | Nudge | Per-deck: `a`=D1 fwd, `z`=D1 bwd, `d`=D2 fwd, `c`=D2 bwd; upper key = forward |
| 14 | Nudge mode toggle | Global: `Shift+\` (`\|`) |
| 15 | Tempo reset | `Shift+'` (D1), `Shift+#` (D2) |
| 16 | Tick offset | Per-deck: `Shift+1`=D1 offset+, `Shift+q`=D1 offset−, `Shift+3`=D2 offset+, `Shift+e`=D2 offset− |
| 17 | Zoom | Global: plain `-` (in), plain `=` (out) |
| 18 | Detail height | Global: `Shift+[` (decrease), `Shift+]` (increase) |
| 19 | Latency | Global: plain `[` (decrease), plain `]` (increase) |
| 20 | Terminal refresh | Global: plain `` ` `` |
| 21 | Deck hide | Removed — two decks always visible |
| 22 | Waveform style | Global: `Shift+O` |
| 23 | Palette cycle | Global: `Shift+P` |

## Unresolved

- `5`, `6`, `9`, `0` — number row, all layers free
- `t`, `y` — top row, all layers free
- `o`, `p` — top row, plain/Space free (Shift = WVFM/PALT)
- `g`, `h`, `l`, `;` — home row, all layers free
- `\` — bottom row, plain/Space free
- `.` — bottom row, all layers free
- Various Space slots: `s`, `d`, `f`, `x`, `v`, `b`, `n`, `/`

---

## Draft Layout

Each key cell shows three layers:

```
┌───────┐
│ SHIFT │  ← Shift+key
│ plain │  ← unmodified
│ SPACE │  ← Space+key
└───────┘
```

Abbreviations:

```
1/2       Deck 1 / Deck 2
P/P       Play/Pause          CUE       Cue point
BRW       Load browser        NMOD      Nudge mode toggle (global)
N>  N<    Nudge fwd / bwd
TAP       BPM tap             MTR       Metronome
DET       BPM auto-detect     T=        Tempo reset
LV+ LV-   Level up / down     MAX MIN   Level snap max / min
BM+ BM-   Playback BPM ±0.1   bm+ bm-   Base BPM ±0.01
HPF       Filter cut bass     LPF       Filter cut treble
F=        Filter reset
+4b +8b   Jump +4 / +8 bars   -4b -8b   Jump -4 / -8 bars
+1Bt      Jump +1 beat        +4Bt      Jump +4 beats
ZMIN ZMOT Zoom in / out       HGT±      Detail height
LAT±      Audio latency       OF+  OF-  Tick offset + / −
WVFM      Waveform style      PALT      Palette cycle
TREF      Terminal refresh    HELP      Help    QUIT      Quit
```

```
── NUMBER ROW ────────────────────────────────────────────────────────────────────────────────
 keys       │  `  │  1  │  2  │  3  │  4  │  5  │  6  │  7  │  8  │  9  │  0  │  -  │  =  │
 Sh         │     │1OF+ │     │2OF+ │     │     │     │     │     │     │     │     │     │
 --         │TREF │1+4b │1+8b │2+4b │2+8b │     │     │1HPF │2LPF │     │     │ZMIN │ZMOT │
 Sp         │     │1+1Bt│1+4Bt│2+1Bt│2+4Bt│     │     │1 F= │2 F= │     │     │     │     │
── TOP ROW ───────────────────────────────────────────────────────────────────────────────────
 keys             │  q  │  w  │  e  │  r  │  t  │  y  │  u  │  i  │  o  │  p  │  [  │  ]  │
 Sh               │1OF- │     │2OF- │     │     │     │     │     │WVFM │PALT │HGT- │HGT+ │
 --               │1-4b │1-8b │2-4b │2-8b │     │     │1LPF │2HPF │     │     │LAT- │LAT+ │
 Sp               │1-1Bt│1-4Bt│2-1Bt│2-4Bt│     │     │1 F= │2 F= │     │     │     │     │
── HOME ROW ──────────────────────────────────────────────────────────────────────────────────
 keys             │  a  │  s  │  d  │  f  │  g  │  h  │  j  │  k  │  l  │  ;  │  '  │  #  │
 Sh               │     │1 bm-│     │2 bm-│     │     │     │     │     │     │1 T= │2 T= │
 --               │1 N> │1 BM-│2 N> │2 BM-│     │     │1 LV+│2 LV+│     │     │1 DET│2 DET│
 Sp               │1 CUE│     │2 CUE│     │     │     │1 MAX│2 MAX│     │     │     │     │
── BOTTOM ROW ────────────────────────────────────────────────────────────────────────────────
 keys       │  \  │  z  │  x  │  c  │  v  │  b  │  n  │  m  │  ,  │  .  │  /  │
 Sh         │NMOD │1 BRW│1 bm+│2 BRW│2 bm+│1 MTR│2 MTR│     │     │     │HELP │
 --         │     │1 N< │1 BM+│2 N< │2 BM+│1 TAP│2 TAP│1 LV-│2 LV-│     │     │
 Sp         │     │1 P/P│     │2 P/P│     │     │     │1 MIN│2 MIN│     │     │

  ESC = QUIT (global)
```
