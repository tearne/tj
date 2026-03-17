# Design: Keymap Redesign
**Status: Complete**

## Approach

Treat two-deck operation as the primary model. All per-deck controls are independently accessible at all times — no deck-select key required. Deck hide removed.

### Modifier convention

- **Space** — in-set actions (play/pause, cue, beat jumps, level snap, filter reset)
- **Shift** — configuration actions (base BPM, browser load, metronome, nudge mode, waveform/palette)

### Removed bindings

- `deck_select_a` / `deck_select_b` (`g` / `h`)
- `deck_a_hide` / `deck_b_hide` (`G` / `H`)

### New / changed bindings (config.toml)

All per-deck actions are now duplicated for deck 1 and deck 2. UK keyboard layout — Shift variants encoded as the shifted character (e.g. `Shift+'` → `@`, `Shift+#` → `~`, `Shift+3` → `£`).

```toml
[keys]
quit             = "esc"

# Global
zoom_in          = "-"
zoom_out         = "="
height_increase  = "}"          # Shift+]
height_decrease  = "{"          # Shift+[
latency_decrease = "["
latency_increase = "]"
waveform_style   = "O"          # Shift+o
palette_cycle    = "P"          # Shift+p
terminal_refresh = "`"
help             = "?"          # Shift+/
nudge_mode_toggle = "|"         # Shift+\

# Deck 1
deck1_play_pause        = "space+z"
deck1_open_browser      = "Z"           # Shift+z
deck1_bpm_tap           = "b"
deck1_metronome         = "B"           # Shift+b
deck1_redetect_bpm      = "'"
deck1_tempo_reset       = "@"           # Shift+' (UK)
deck1_bpm_increase      = "s"
deck1_bpm_decrease      = "x"
deck1_base_bpm_increase = "S"           # Shift+s
deck1_base_bpm_decrease = "X"           # Shift+x
deck1_nudge_forward     = "a"
deck1_nudge_backward    = "z"
deck1_offset_increase   = "!"           # Shift+1
deck1_offset_decrease   = "Q"           # Shift+q
deck1_level_up          = "j"
deck1_level_down        = "m"
deck1_level_max         = "space+j"
deck1_level_min         = "space+m"
deck1_filter_increase   = "7"           # HPF (cut bass)
deck1_filter_decrease   = "u"           # LPF (cut treble)
deck1_filter_reset      = ["space+7", "space+u"]
deck1_jump_forward_4b   = "1"
deck1_jump_backward_4b  = "q"
deck1_jump_forward_8b   = "2"
deck1_jump_backward_8b  = "w"
deck1_jump_forward_1bt  = "space+1"
deck1_jump_backward_1bt = "space+q"
deck1_jump_forward_4bt  = "space+2"
deck1_jump_backward_4bt = "space+w"

# Deck 2
deck2_play_pause        = "space+c"
deck2_open_browser      = "C"           # Shift+c
deck2_bpm_tap           = "n"
deck2_metronome         = "N"           # Shift+n
deck2_redetect_bpm      = "#"
deck2_tempo_reset       = "~"           # Shift+# (UK)
deck2_bpm_increase      = "f"
deck2_bpm_decrease      = "v"
deck2_base_bpm_increase = "F"           # Shift+f
deck2_base_bpm_decrease = "V"           # Shift+v
deck2_nudge_forward     = "d"
deck2_nudge_backward    = "c"
deck2_offset_increase   = "£"           # Shift+3 (UK)
deck2_offset_decrease   = "E"           # Shift+e
deck2_level_up          = "k"
deck2_level_down        = ","
deck2_level_max         = "space+k"
deck2_level_min         = "space+,"
deck2_filter_increase   = "i"           # HPF (cut bass)
deck2_filter_decrease   = "8"           # LPF (cut treble)
deck2_filter_reset      = ["space+i", "space+8"]
deck2_jump_forward_4b   = "3"
deck2_jump_backward_4b  = "e"
deck2_jump_forward_8b   = "4"
deck2_jump_backward_8b  = "r"
deck2_jump_forward_1bt  = "space+3"
deck2_jump_backward_1bt = "space+e"
deck2_jump_forward_4bt  = "space+4"
deck2_jump_backward_4bt = "space+r"
```

### Final layout

Each key cell shows three layers:
```
┌───────┐
│ SHIFT │  ← Shift+key
│ plain │  ← unmodified
│ SPACE │  ← Space+key
└───────┘
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
  Space+A = Deck 1 cue  /  Space+D = Deck 2 cue  (reserved; implemented via cue proposal)
```
