# Config

## Player Controls

Controls are split into two groups. The **mixer** — level, gain, and filter — addresses each deck directly via fixed key columns. Everything else operates on the **selected deck**, chosen with `Space+1` or `Space+2`.

### Keyboard Layout

```
── DECK CONTROLS (selected deck: Space+1 / Space+2) ───────────────────────

      │     1     │     2     │     3     │     4     │     5     │
Shift │           │           │           │  +32 bars │ +64 bars  │
   -- │  +1 beat  │  +1 bar   │  +4 bars  │  +8 bars  │ +16 bars  │
Space │ Select D1 │ Select D2 │           │           │           │
──────┼───────────┼───────────┼───────────┼───────────┼───────────┤
      │     Q     │     W     │     E     │     R     │     T     │
Shift │           │           │           │  -32 bars │ -64 bars  │
   -- │  -1 beat  │  -1 bar   │  -4 bars  │  -8 bars  │ -16 bars  │
Space │           │           │  Cue Set  │  Cue Jump │           │
──────┼───────────┼───────────┼───────────┼───────────┼───────────┤
      │     A     │     S     │     D     │     F     │     G     │
Shift │           │           │           │  Tick +   │ Bse BPM - │
   -- │  Pitch -  │   PFL +   │           │ Nudge Fwd │   BPM -   │
Space │  Pitch =  │ PFL Reset │  Browse   │ Play/Pause│ PFL On/Off│
──────┼───────────┼───────────┼───────────┼───────────┼───────────┤
      │     Z     │     X     │     C     │     V     │     B     │
Shift │           │           │           │  Tick -   │ Bse BPM + │
   -- │  Pitch +  │   PFL -   │  BPM Tap  │ Nudge Bck │   BPM +   │
Space │  Pitch =  │ PFL Reset │ BPM Detect│           │ Metronome │



── MIXER (D1 / D2) ─────────────────────────────────

      │     7     │     8     │
Shift │Slope +    │Slope +    │
   -- │ Filter HPF│ Filter HPF│
Space │  Filter = │  Filter = │
──────┼───────────┼───────────┤
      │     U     │     I     │
Shift │Slope -    │Slope -    │
   -- │ Filter LPF│ Filter LPF│
Space │  Filter = │  Filter = │
──────┼───────────┼───────────┤
      │     J     │     K     │
Shift │  Gain +   │  Gain +   │
   -- │  Level +  │  Level +  │
Space │ Level 100%│ Level 100%│
──────┼───────────┼───────────┤
      │     M     │     ,     │
Shift │  Gain -   │  Gain -   │
   -- │  Level -  │  Level -  │
Space │ Level  0% │ Level  0% │



── GLOBAL ──────────────────────────────────────────────────────────────────────

      │     `     │
Shift │ Nudge Mode│
   -- │   Vinyl   │
Space │           │


      │     -     │     =     │
Shift │           │           │
   -- │  Zoom In  │  Zoom Out │
Space │           │           │
──────┼───────────┼───────────┤
      │     [     │     ]     │
Shift │  Height - │  Height + │
   -- │ Latency - │ Latency + │
Space │           │           │
──────┼───────────┼───────────┤
      │     #     │     /     │
Shift │  Palette  │   Help    │
   -- │           │ Album Art │
Space │           │  Kbd Map  │



── LEGEND ─────────────────────────────────────────────────────────────────────

  Select D1/2    select deck            Play/Pause     play/pause
  Browse         file browser           Pitch +/-      ±1 semitone
  Pitch =        pitch reset            Nudge Fwd/Bck  nudge fwd/back
  BPM +/-        BPM ±0.1               BPM Tap        tap BPM
  BPM Detect     BPM re-detect          Bse BPM +/-    base BPM ±0.01
  Tick +/-       tick offset            Metronome      metronome toggle
  PFL +/-        PFL level ±0.20        PFL Reset      PFL level reset
  PFL On/Off     PFL toggle             Cue Jump       jump to cue point
  Cue Set        set cue point          Level +/-      fader ±0.05
  Level 100%/0%  level max/min          Gain +/-       gain trim ±1 dB
  Filter HPF/LPF toward HPF/LPF         Filter =       filter flat
  Vinyl          vinyl mode             Nudge Mode     nudge mode toggle
  Zoom In/Out    zoom                   Latency +/-    latency ±10ms
  Height +/-     waveform height        Refresh        refresh terminal
  Help           key help               Album Art      album art toggle
  Kbd Map        keyboard layout        Palette        palette cycle (¬=Nudge Mode, Shift+#=Palette on UK keyboard)
```

Keys `6`, `Y`, `H`, and `N` are intentionally unbound.

## Config Loading

- Key bindings are loaded from `config.toml` at startup — first from the same directory as the binary, then from `~/.config/deck/config.toml`. If neither file is found, the embedded default config is written to `~/.config/deck/config.toml` and loaded automatically.
- Bindings are declared under a `[keys]` table as `function_name = "key_string"` or `function_name = ["key1", "key2"]` for multiple keys per function.
- Key strings: printable characters as-is (`q`, `+`, `H`); special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`); `space+<key>` for Space-modifier chords (e.g. `space+f`).
- `Space` acts as a modifier key: holding it and pressing another key fires a chord action. `Space` released alone has no effect. The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.
- Ctrl-C always quits unconditionally and is not configurable.
- Display parameters are declared under a `[display]` table. Missing `[display]` keys fall back to their defaults; existing config files are never modified automatically.
