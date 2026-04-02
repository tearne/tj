# Keyboard Layout

## Intent
The current layout assigns every control to a fixed per-deck key, which works for two decks but does not scale. This change introduces a **selected deck** model and reorganises the layout around it.

Controls are split into two groups. The **mixer** — level, gain, and filter — addresses each deck directly via fixed key columns, keeping hands on the mixer during a mix. Everything else — play/pause, pitch, nudge, BPM, cue, browse, PFL — operates on the **selected deck**, chosen by `Space+1` or `Space+2`, so a single set of keys serves both decks.

Additional changes included in this layout:

- Beat jumps extended to ±32 and ±64 bars
- PFL becomes a level control (up/down/reset) rather than a toggle
- BPM tap moved to `C`; metronome toggle moved to `Space+B`
- Tick (beat phase offset) and base BPM adjustments operate on the selected deck
- Cue set and cue jump operate on the selected deck
- Help moved to `Shift+/`; zoom on `-`/`=`; latency on `[`/`]`; waveform height on `Shift+[`/`Shift+]`

## Approach

The selected deck is a new runtime state, defaulting to deck 1 on startup. `Space+1`/`Space+2` sets it. All deck control functions that previously took a fixed deck argument are updated to read the selected deck instead.

PFL changes from a boolean toggle to a float level (0.0–1.0), stepping ±0.20 per keypress. The existing PFL toggle (`pfl_toggle`) is retained as `pfl_on_off`, bound to `Space+G`. New functions `pfl_level_up`, `pfl_level_down`, and `pfl_level_reset` are added.

Cue functions are unchanged internally; their bindings move from the old per-deck keys to the selected-deck keys (`Space+E` / `Space+R`).

The extended jump sizes (±32b, ±64b) are new binding entries pointing to existing jump logic with new bar counts.

Selected-deck controls require a structural change to the `Action` enum: per-deck variants such as `Deck1PlayPause`/`Deck2PlayPause`, `Deck1OpenBrowser`/`Deck2OpenBrowser`, and equivalent pairs for pitch, nudge, BPM, cue, and metronome are replaced by single deck-agnostic variants (`PlayPause`, `OpenBrowser`, etc.) that operate on the selected deck at runtime. Mixer actions (`Deck1LevelUp`, `Deck2LevelUp`, etc.) retain explicit deck numbers as they are addressed directly.

The selected deck is shown in the existing deck notification bar: the deck number is inset one space from the left edge and highlighted yellow.

`album_art_toggle` moves to `/` (unmodified). `palette_cycle` moves to `Shift+#`.

The embedded default `config.toml` is updated with all new bindings.

Review cadence: at the end.

`SPEC/config.md` is updated to replace the current tables and keyboard diagram with the layout below.

Keys `6`, `Y`, `H`, and `N` are intentionally unbound.

### Layout

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
Space │           │           │  Cue Jump │  Cue Set  │           │
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
Shift │           │           │
   -- │ Filter HPF│ Filter HPF│
Space │  Filter = │  Filter = │
──────┼───────────┼───────────┤
      │     U     │     I     │
Shift │           │           │
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

── Number row ──
      │     `     │...│     -     │     =     │
Shift │ Nudge Mode│   │           │           │
   -- │   Vinyl   │   │  Zoom Out │  Zoom In  │
Space │  Refresh  │   │           │           │

── Top row ──
      │     [     │     ]     │
Shift │  Height - │  Height + │
   -- │ Latency - │ Latency + │
Space │           │           │

── Home / bottom row ──
      │     #     │     /     │
Shift │  Palette  │   Help    │
   -- │           │ Album Art │
Space │           │           │


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
  Palette        palette cycle (¬=Nudge Mode, Shift+#=Palette on UK keyboard)
```

## Plan

- [x] UPDATE SPEC — `SPEC/config.md`: replace player controls tables and keyboard diagram with the new layout
- [x] UPDATE SPEC — `SPEC/deck.md`: document selected deck state and PFL as a float level field
- [x] UPDATE SPEC — `SPEC/render.md`: document selected deck indicator (inset one space, yellow highlight)
- [x] REVIEW `Action` enum variants and all handler sites to confirm scope before restructuring
- [x] UPDATE IMPL — Restructure `Action` enum: remove per-deck selected-deck variants; add deck-agnostic equivalents (`PlayPause`, `OpenBrowser`, `PitchUp`, etc.) and `SelectDeck1`/`SelectDeck2`
- [x] ADD IMPL — Add `selected_deck` runtime state (default deck 1); route all deck-agnostic action handlers through it
- [x] UPDATE IMPL — PFL: replace boolean toggle with float level (0.0–1.0, ±0.20 step); add `PflLevelUp`, `PflLevelDown`, `PflLevelReset`, `PflOnOff` actions
- [x] ADD IMPL — Extended jumps: add ±32-bar and ±64-bar jump action variants
- [x] UPDATE IMPL — Move `AlbumArtToggle` binding to `/`; move `PaletteCycle` binding to `Shift+#`
- [x] UPDATE RENDER — Selected deck indicator: inset one space from left edge, highlight yellow
- [x] UPDATE IMPL — Update embedded default `config.toml` with all new bindings
- [x] ADD IMPL — Build release binary (`cargo build --release`)
- [x] TEST — Verify all new bindings, selected deck behaviour, PFL level control, and extended jumps against `SPEC/config.md`

## Log

- Filter slope keys were omitted from the initial layout; added after conclusion. Slope + on Shift+7/8 (`&`/`*`), Slope − on Shift+U/I (`U`/`I`). Updated `SPEC/config.md` mixer table and `resources/config.toml`.
- Space+f double-fired nudge_forward and play_pause; fixed by adding `!space_held` guard to both nudge Press/Repeat handlers.
- `terminal_refresh` removed entirely — action, handler, and config entry deleted. `TerminalRefresh` variant removed from `Action` enum.

## Conclusion

All implementation tasks complete. The `Action` enum was restructured to remove 40+ per-deck selected-deck variants, replacing them with deck-agnostic equivalents routed through a new `selected_deck: usize` state (default 0). PFL changed from a boolean toggle to a float level (0–100 internally, ±20 per step). Extended jumps (16b, 32b, 64b) were added for both beat and vinyl mode. The deck number label in the notification bar is now yellow when that deck is selected. `config.toml` was fully rewritten to match the new layout. The `PflLevelUp` handler required restructuring to avoid simultaneous mutable borrows on `decks[selected]` and `decks[other]`. The release binary builds cleanly. Manual verification on host required.
