# Config

## Player Controls

**Deck 1 controls:**
| Key | Action |
|-----|--------|
| `Space+z` | Play / pause |
| `Z` | Open file browser |
| `1` / `q` | Jump ±4 bars |
| `2` / `w` | Jump ±8 bars |
| `Space+1` / `Space+q` | Jump ±1 beat |
| `Space+2` / `Space+w` | Jump ±4 beats |
| `a` / `z` | Nudge forward / backward |
| `j` / `m` | Level up / down |
| `Space+j` / `Space+m` | Level 100% / 0% |
| `J` / `M` | Gain trim +1 dB / −1 dB |
| `7` / `u` | Filter toward HPF / LPF |
| `Space+7` / `Space+u` | Filter reset to flat |
| `x` / `s` | BPM +0.1 / −0.1 |
| `X` / `S` | Base BPM +0.01 / −0.01 |
| `@` | Tempo reset |
| `b` / `B` | Tap BPM / metronome toggle |
| `'` | BPM re-detect (suppressed in vinyl mode) |
| `!` / `Q` | Beat phase offset +10ms / −10ms |
| `A` | Cue set |
| `Space+a` | Cue play |
| `Space+x` | PFL toggle |

**Deck 2 controls:**
| Key | Action |
|-----|--------|
| `Space+c` | Play / pause |
| `C` | Open file browser |
| `3` / `e` | Jump ±4 bars |
| `4` / `r` | Jump ±8 bars |
| `Space+3` / `Space+e` | Jump ±1 beat |
| `Space+4` / `Space+r` | Jump ±4 beats |
| `d` / `c` | Nudge forward / backward |
| `k` / `,` | Level up / down |
| `Space+k` / `Space+,` | Level 100% / 0% |
| `K` / `<` | Gain trim +1 dB / −1 dB |
| `8` / `i` | Filter toward HPF / LPF |
| `Space+8` / `Space+i` | Filter reset to flat |
| `v` / `f` | BPM +0.1 / −0.1 |
| `V` / `F` | Base BPM +0.01 / −0.01 |
| `~` | Tempo reset |
| `n` / `N` | Tap BPM / metronome toggle |
| `#` | BPM re-detect (suppressed in vinyl mode) |
| `£` / `E` | Beat phase offset +10ms / −10ms |
| `D` | Cue set |
| `Space+d` | Cue play |
| `Space+v` | PFL toggle |

**Global controls:**
| Key | Action |
|-----|--------|
| `[` / `]` | Latency −10ms / +10ms |
| `-` / `=` | Zoom in / out |
| `{` / `}` | Detail height decrease / increase |
| `\|` | Toggle nudge mode: `jump` (10ms seek) / `warp` (±10% speed) |
| `` ` `` | Toggle vinyl mode |
| `¬` | Refresh terminal |
| `?` | Toggle key binding help popup |
| `Esc` / `Ctrl-C` | Quit |

> Key bindings reflect the defaults in `config.toml`. All player bindings are user-configurable.

## Keyboard Layout

The diagram below shows all default bindings across the keyboard. Each cell lists the action for Shift (`Sh`), plain (`--`), and Space-chord (`Sp`) layers.

```
── NUMBER ROW ─────────────────────────────────────────────────
 keys  │  `  │  1  │  2  │  3  │  4  │  7  │  8  │  -  │  =  │
 Sh    │     │1OF+ │     │2OF+ │     │     │     │     │     │
 --    │VNYL │1+4b │1+8b │2+4b │2+8b │1HPF │2LPF │ZMIN │ZMOT │
 Sp    │     │1+1Bt│1+4Bt│2+1Bt│2+4Bt│1 F= │2 F= │     │     │
── TOP ROW ────────────────────────────────────────────────────
 keys  │  q  │  w  │  e  │  r  │  u  │  i  │  o  │  p  │  [  │  ]  │
 Sh    │1OF- │     │2OF- │     │     │     │     │PALT │HGT- │HGT+ │
 --    │1-4b │1-8b │2-4b │2-8b │1LPF │2HPF │     │     │LAT- │LAT+ │
 Sp    │1-1Bt│1-4Bt│2-1Bt│2-4Bt│1 F= │2 F= │     │     │     │     │
── HOME ROW ───────────────────────────────────────────────────
 keys  │  a  │  s  │  d  │  f  │  j  │  k  │  '  │  #  │
 Sh    │1 CUE│1 bm-│2 CUE│2 bm-│1GM+ │2GM+ │1 T= │2 T= │
 --    │1 N> │1 BM-│2 N> │2 BM-│1 LV+│2 LV+│1 DET│2 DET│
 Sp    │1 CPL│     │2 CPL│     │1 MAX│2 MAX│     │     │
── BOTTOM ROW ─────────────────────────────────────────────────
 keys  │  z  │  x  │  c  │  v  │  b  │  n  │  m  │  ,  │
 Sh    │1 BRW│1 bm+│2 BRW│2 bm+│1 MTR│2 MTR│1GM- │2GM- │
 --    │1 N< │1 BM+│2 N< │2 BM+│1 TAP│2 TAP│1 LV-│2 LV-│
 Sp    │1 P/P│1PFL │2 P/P│2PFL │     │     │1 MIN│2 MIN│
ESC = QUIT
```

## Config Loading

- Key bindings are loaded from `config.toml` at startup — first from the same directory as the binary, then from `~/.config/deck/config.toml`. If neither file is found, the embedded default config is written to `~/.config/deck/config.toml` and loaded automatically.
- Bindings are declared under a `[keys]` table as `function_name = "key_string"` or `function_name = ["key1", "key2"]` for multiple keys per function.
- Key strings: printable characters as-is (`q`, `+`, `H`); special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`); `space+<key>` for Space-modifier chords (e.g. `space+z`).
- `Space` acts as a modifier key: holding it and pressing another key fires a chord action. `Space` released alone has no effect. The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.
- Ctrl-C always quits unconditionally and is not configurable.
- Display parameters are declared under a `[display]` table. Missing `[display]` keys fall back to their defaults; existing config files are never modified automatically.
