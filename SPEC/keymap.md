# Keymap

## Player Controls

**Active-deck controls** (apply to whichever deck is currently selected):
| Key | Action |
|-----|--------|
| `Space+Z` | Play / Pause |
| `Space+F` / `Space+V` | Reset tempo to detected BPM (speed → 1×) |
| `+` / `_` | Adjust beat phase offset (10ms steps) |
| `[` / `]` | Latency ±10ms (live adjustment; also compensates `offset_ms` to keep ticks anchored) |
| `Left` / `Right` | Seek backward / forward (small increment, e.g. 5s) |
| `1` / `2` / `3` / `4` | Beat jump forward 1 / 4 / 16 / 64 beats |
| `q` / `w` / `e` / `r` | Beat jump backward 1 / 4 / 16 / 64 beats |
| `c` / `d` | Nudge backward / forward (mode-dependent) |
| `C` / `D` | Toggle nudge mode: `jump` (10ms seek) / `warp` (±10% speed) |
| `-` / `=` | Zoom in / out |
| `{` / `}` | Detail height decrease / increase |
| `v` / `f` | BPM +0.01 / −0.01 |
| `V` / `F` | Base BPM +0.01 / −0.01 |
| `b` | Tap BPM detection |
| `'` | Toggle metronome |
| `@` | Trigger manual BPM re-detection |

**Per-deck fixed controls** (always apply to the named deck, regardless of which is active):
| Key | Action |
|-----|--------|
| `j` / `m` | Deck A level up / down |
| `Space+J` / `Space+M` | Deck A level 100% / 0% |
| `u` / `7` | Deck A filter sweep: `u` toward LPF, `7` toward HPF |
| `Space+u` / `Space+7` | Deck A snap filter to flat |
| `k` / `,` | Deck B level up / down |
| `i` / `8` | Deck B filter sweep: `i` toward LPF, `8` toward HPF |

**Global controls** (not deck-specific):
| Key | Action |
|-----|--------|
| `g` | Select Deck A as active |
| `h` | Select Deck B as active |
| `z` | Open / close file browser (loads into active deck) |
| `?` | Toggle key binding help popup |
| `` ` `` | Refresh terminal (clear display glitches) |
| `Esc` / `Ctrl-C` | Quit |

> Key bindings reflect the defaults in `config.toml`. All player bindings are user-configurable.

## Keyboard Layout

The diagram below shows all default bindings across the keyboard. Each cell lists the action for Shift (`Sh`), plain (`--`), and Space-chord (`Sp`) layers.

```
── NUMBER ROW ─────────────────────────────────────────────────
 keys  │  `  │  1  │  2  │  3  │  4  │  7  │  8  │  -  │  =  │
 Sh    │     │1OF+ │     │2OF+ │     │     │     │     │     │
 --    │TREF │1+4b │1+8b │2+4b │2+8b │1HPF │2LPF │ZMIN │ZMOT │
 Sp    │     │1+1Bt│1+4Bt│2+1Bt│2+4Bt│1 F= │2 F= │     │     │
── TOP ROW ────────────────────────────────────────────────────
 keys  │  q  │  w  │  e  │  r  │  u  │  i  │  o  │  p  │  [  │  ]  │
 Sh    │1OF- │     │2OF- │     │     │     │WVFM │PALT │HGT- │HGT+ │
 --    │1-4b │1-8b │2-4b │2-8b │1LPF │2HPF │     │     │LAT- │LAT+ │
 Sp    │1-1Bt│1-4Bt│2-1Bt│2-4Bt│1 F= │2 F= │     │     │     │     │
── HOME ROW ───────────────────────────────────────────────────
 keys  │  a  │  s  │  d  │  f  │  j  │  k  │  '  │  #  │
 Sh    │     │1 bm-│     │2 bm-│     │     │1 T= │2 T= │
 --    │1 N> │1 BM-│2 N> │2 BM-│1 LV+│2 LV+│1 DET│2 DET│
 Sp    │1 CUE│     │2 CUE│     │1 MAX│2 MAX│     │     │
── BOTTOM ROW ─────────────────────────────────────────────────
 keys  │  z  │  x  │  c  │  v  │  b  │  n  │  m  │  ,  │
 Sh    │1 BRW│1 bm+│2 BRW│2 bm+│1 MTR│2 MTR│     │     │
 --    │1 N< │1 BM+│2 N< │2 BM+│1 TAP│2 TAP│1 LV-│2 LV-│
 Sp    │1 P/P│     │2 P/P│     │     │     │1 MIN│2 MIN│
ESC = QUIT
```

## Config Loading

- Key bindings are loaded from `config.toml` at startup — first from the same directory as the binary, then from `~/.config/tj/config.toml`. If neither file is found, the embedded default config is written to `~/.config/tj/config.toml` and loaded automatically.
- Bindings are declared under a `[keys]` table as `function_name = "key_string"` or `function_name = ["key1", "key2"]` for multiple keys per function.
- Key strings: printable characters as-is (`q`, `+`, `H`); special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`); `space+<key>` for Space-modifier chords (e.g. `space+z`).
- `Space` acts as a modifier key: holding it and pressing another key fires a chord action. `Space` released alone has no effect. The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.
- Ctrl-C always quits unconditionally and is not configurable.
- Display parameters are declared under a `[display]` table. Missing `[display]` keys fall back to their defaults; existing config files are never modified automatically.
