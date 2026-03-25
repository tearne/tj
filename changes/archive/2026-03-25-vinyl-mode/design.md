# Design: Vinyl Mode
**Status: Approved** *(retrospective)*

## Approach

Vinyl mode is a single `bool` living in the TUI loop (`vinyl_mode`) and persisted in `CacheFile`. No new abstractions are needed — the existing speed, jump, and rendering paths are branched at the call site.

**Speed (`vinyl_speed: f32` on `TempoState`)** — An absolute multiplier (1.0 = nominal), independent of `bpm`/`base_bpm`. In vinyl mode, the BPM adjust keys write `vinyl_speed` and call `player.set_speed(vinyl_speed)` directly. On vinyl → beat transition, `bpm = base_bpm × vinyl_speed`; on beat → vinyl transition, `vinyl_speed = bpm / base_bpm`. Neither transition changes the audio speed.

**Tick and cue suppression** — The background renderer receives `analysing = true` (which zeroes the tick row) and `cue_sample = None` whenever vinyl mode is active. This is applied at the `store_tempo`/`store_cue` call site in the render loop — no changes to `SharedDetailRenderer` are needed.

**Display** — `info_line_for_deck` takes a new `vinyl_mode: bool` parameter. A single `show_percentage` flag (true when `vinyl_mode || !bpm_established`) selects between the percentage branch and the BPM branch. BPM display is rounded to 1dp throughout beat mode.

**Time jumps** — `do_time_jump` added to `deck` module alongside `do_jump`. Beat jump match arms branch on `vinyl_mode` inline.

**Display smoothing** — `service_deck_frame` takes `vinyl_mode: bool` and substitutes `vinyl_speed` for `bpm / base_bpm` in the smooth position advance and drift correction calculations.

## Tasks

1. ✓ Impl: `vinyl_speed: f32` and `do_time_jump` in `src/deck/mod.rs`
2. ✓ Impl: `vinyl_mode` persistence in `src/cache/mod.rs`
3. ✓ Impl: `VinylModeToggle` action in `src/config/mod.rs` and `resources/config.toml`
4. ✓ Impl: info bar percentage display and 1dp BPM rounding in `src/render/mod.rs`
5. ✓ Impl: `vinyl_mode` state, toggle handler, BPM key branching, jump remapping, redetect guard, tick/cue suppression, detail info bar label, display smoothing in `src/main.rs`
6. ✓ Spec: `SPEC/deck.md`, `SPEC/render.md`, `SPEC/config.md`, `SPEC/cache.md` updated
