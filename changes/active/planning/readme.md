# Readme

## Intent
The project has no README. New users have no introduction to what `deck` is, no way to install or run it, and no quick reference for the key bindings.

The README should cover:

- **Rationale** — modern DJ equipment does so much it removes the fun. `deck` blends the convenience of software with the skill of beat-matching and mixing; you shouldn't need to buy turntables, vinyl and an analogue mixer to get the real-time experience.
- **Design philosophy** — two decks; TUI; nudge and playback speed adjustment; unified high-pass/low-pass filter; BPM optional (used only for beat jump, set manually or by tapping). Deliberately excludes loops, effects, jump points, samples, and track recommendations.
- **Screenshot**
- **Default key bindings** — ASCII-art keyboard diagram
- **Linux installation** and runtime dependencies; the binary is `deck`
- **Attribution** of key dependencies

## Approach

A single `README.md` at the project root. Sections in order:

1. **Name and one-liner** — `deck` — a minimal terminal DJ player
2. **Screenshot** — a terminal screenshot stored as `screenshot.png` in the repo root, embedded with a relative image link
3. **Rationale** — the two-sentence pitch from the Intent, verbatim or lightly edited
4. **Design** — brief bullet list from the Intent (TUI, two decks, nudge, filter, optional BPM, deliberate omissions)
5. **Key bindings** — a short prose explanation first: Shift gives uppercase keys as usual; Space is a held modifier — hold Space, press another key to fire a chord action. Then the keyboard layout table and legend from `SPEC/config.md`, copied verbatim; no need to maintain two sources at this stage
6. **Installation** — `cargo build --release`; copy binary to PATH; runtime dependency: ALSA or PipeWire
7. **Attribution** — one line each for `symphonia` (decode), `rodio` (playback), `ratatui` + `crossterm` (TUI), `stratum-dsp` (BPM detection), `lofty` (tag read/write)

The screenshot must be taken of a running instance with both decks loaded and the waveforms visible.

## Plan

- [x] REVIEW verify `SPEC/config.md` keyboard layout is current against `src/config/mod.rs` defaults; flag any discrepancies before proceeding
- [x] ADD key bindings section only — Space/Shift prose explanation followed by the keyboard diagram and legend — and surface for user review
- [ ] ADD `README.md` at project root with all remaining sections: name/one-liner, screenshot placeholder, rationale, design, installation, attribution
- [ ] ADD screenshot of a running instance (both decks loaded, waveforms visible) saved as `screenshot.png` at repo root; replace placeholder link

Review cadence: after the key bindings section (before continuing), then at the end.

## Feedback

**Delivery status**: not delivered

The approach of copying the `SPEC/config.md` keyboard diagram verbatim was set aside during review in favour of a per-deck layout intended to build a clearer conceptual model for new users. Several iterations were explored but no settled design was reached, and the build was stopped before any other README sections were written.

The planner should reconsider the key bindings section design before this change is restarted. Specifically:

- Whether to use a per-deck grid or the existing full-keyboard diagram from `SPEC/config.md`
- If per-deck: the exact cell content for each key position (Sh / plain / Space layers)

As a side-effect of the REVIEW task, a label error in `SPEC/config.md` was fixed: the `8` and `i` filter direction labels for deck 2 were swapped in the keyboard diagram (now corrected to `8`→HPF, `i`→LPF).
