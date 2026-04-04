# Readme

## Intent

The project has no README. New users have no introduction to what `deck` is, no way to install or run it, and no quick reference for the key bindings.

The README should cover:

- **Rationale** — modern DJ equipment does so much it removes the fun. `deck` blends the convenience of software with the skill of beat-matching and mixing; you shouldn't need to buy turntables, vinyl and an analogue mixer to get the real-time experience.
- **Features** — three decks; TUI; nudge and playback speed adjustment; unified high-pass/low-pass filter; BPM optional (used only for beat jump, set manually or by tapping). Deliberately excludes loops, effects, jump points, samples, and track recommendations.
- **Screenshot**
- **Default key bindings** — ASCII-art keyboard diagram
- **Linux installation** and runtime dependencies; the binary is `deck`
- **Attribution** of key dependencies

## Approach

A single `README.md` at the project root. Sections in order:

1. **Name and one-liner** — `deck` — a minimal terminal DJ player
2. **Screenshot** — a terminal screenshot stored as `screenshot.png` in the repo root, embedded with a relative image link
3. **Rationale** — the two-sentence pitch from the Intent, verbatim or lightly edited
4. **Features** — brief bullet list: TUI, three decks, nudge and playback speed adjustment, unified HPF/LPF filter, optional BPM (beat jump, set manually or by tapping), deliberate omissions (no loops, effects, jump points, samples, track recommendations)
5. **Key bindings** — a short prose explanation first: Shift gives uppercase keys as usual; Space is a held modifier — hold Space, press another key to fire a chord action. Then the keyboard layout diagram and legend copied verbatim from `SPEC/config.md`. No redesign — the existing diagram already matches the in-app overlay and is maintained there.
6. **Installation** — `cargo build --release`; copy binary to PATH; runtime dependency: ALSA or PipeWire
7. **Attribution** — one line each for `symphonia` (decode), `rodio` (playback), `ratatui` + `crossterm` (TUI), `stratum-dsp` (BPM detection), `lofty` (tag read/write)

The screenshot must be taken of a running instance with both decks loaded and the waveforms visible.

Review cadence: at the end.

## Plan

- [x] REVIEW verify `SPEC/config.md` keyboard layout is current against `src/config/mod.rs` defaults; flag any discrepancies before proceeding
- [ ] ADD `README.md` at project root with all sections: name/one-liner, screenshot placeholder, rationale, design, key bindings (prose + verbatim diagram and legend from `SPEC/config.md`), installation, attribution
- [ ] ADD screenshot of a running instance (both decks loaded, waveforms visible) saved as `screenshot.png` at repo root; replace placeholder link

## Conclusion
