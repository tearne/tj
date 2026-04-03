# Config

## Player Controls

Controls are split into two groups. The **mixer** — level, gain, and filter — addresses each deck directly via fixed key columns. Everything else operates on the **selected deck**, chosen with `Space+1` or `Space+2`.

### Keyboard Layout

The layout below matches the in-app keyboard help overlay (`Space+/`). Left block = deck controls (selected deck); right block = mixer (addressed directly). Keys `6`, `Y`, `H`, and `N` are intentionally unbound.

```
╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭ +Slp    ╭ +Slp
1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7 HPF     8 HPF
╰ SelD1   ╰ SelD2   ╰         ╰         ╰         ┆   ╰ Flt=    ╰ Flt=
  ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭ -Slp    ╭ -Slp
  Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U LPF     I LPF
  ╰         ╰         ╰ CueSt   ╰ CueJp   ╰         ┆   ╰ Flt=    ╰ Flt=
    ╭         ╭         ╭ +Tick   ╭ -BsBPM  ╭         ┆   ╭ +Gain   ╭ +Gain
    A -Ptch   S +PFL    D +Ndge   F -BPM    G         ┆   J +Lvl    K +Lvl
    ╰ -Ptch   ╰ Rst     ╰ Brows   ╰ Play    ╰ PFLTog  ┆   ╰ 100%    ╰ 100%
      ╭         ╭         ╭ -Tick   ╭ +BsBPM  ╭         ┆   ╭ -Gain   ╭ -Gain
      Z +Ptch   X -PFL    C -Ndge   V +BPM    B Tap     ┆   M -Lvl    , -Lvl
      ╰ +Ptch   ╰ Rst     ╰         ╰ Metro   ╰ BDtct   ┆   ╰ 0%      ╰ 0%
```

Per-cell format: `╭ Shift-action` / `Key plain-action` / `╰ Space-action`. Empty modifier cells are left blank.

Global keys (not shown in overlay — see `?` modal):
- `` ` `` vinyl mode, `¬` nudge mode toggle
- `-` / `=` zoom in/out, `{` / `}` waveform height
- `[` / `]` latency ±10ms
- `/` album art, `~` palette cycle, `Space+/` keyboard layout
- `?` help modal, `Esc` quit

```
── LEGEND ─────────────────────────────────────────────────────────────────────

  SelD1/2      select deck            Play         play/pause
  Brows        file browser           -/+Ptch      pitch ±1 semitone
  -Ptch/+Ptch  pitch reset (=)        +/-Ndge      nudge forward/back
  +/-BPM       BPM ±0.1               Tap          tap BPM
  BDtct        BPM re-detect          +/-BsBPM     base BPM ±0.01
  +/-Tick      tick offset            Metro        metronome toggle
  +/-PFL       PFL level ±0.20        Rst          PFL level reset
  PFLTog       PFL toggle             CueJp        jump to cue point
  CueSt        set cue point          +/-Lvl       fader ±0.05
  100%/0%      level max/min          +/-Gain      gain trim ±1 dB
  HPF/LPF      filter toward HPF/LPF  Flt=         filter flat
  +/-Slp       filter slope +/-
```

## Config Loading

- Key bindings are loaded from `config.toml` at startup — first from the same directory as the binary, then from `~/.config/deck/config.toml`. If neither file is found, the embedded default config is written to `~/.config/deck/config.toml` and loaded automatically.
- Bindings are declared under a `[keys]` table as `function_name = "key_string"` or `function_name = ["key1", "key2"]` for multiple keys per function.
- Key strings: printable characters as-is (`q`, `+`, `H`); special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`); `space+<key>` for Space-modifier chords (e.g. `space+f`).
- `Space` acts as a modifier key: holding it and pressing another key fires a chord action. `Space` released alone has no effect. The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.
- Ctrl-C always quits unconditionally and is not configurable.
- Display parameters are declared under a `[display]` table. Missing `[display]` keys fall back to their defaults; existing config files are never modified automatically.
