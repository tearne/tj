# Vinyl Mode
**Type**: Proposal
**Status**: Implemented

## Log

- `src/deck/mod.rs`: added `vinyl_speed: f32` to `TempoState` (init 1.0); added `do_time_jump`
- `src/cache/mod.rs`: added `vinyl_mode: bool` to `CacheFile` and `Cache`; added `get_vinyl_mode`/`set_vinyl_mode`
- `src/config/mod.rs`: added `VinylModeToggle` action
- `resources/config.toml`: `terminal_refresh` → `¬`; `vinyl_mode_toggle` → `` ` ``
- `src/render/mod.rs`: `info_line_for_deck` — percentage display for vinyl/no-BPM; 1dp BPM; beat flash/metronome/offset hidden in vinyl mode
- `src/main.rs`: `vinyl_mode` state variable; `VinylModeToggle` handler with vinyl↔beat conversion; BPM adjust keys branch on vinyl_mode; beat jump keys remap to time-based in vinyl; redetect suppressed in vinyl; store_tempo/store_cue suppress ticks and cue in vinyl; detail info bar always-present `[VINYL]`/`[BEAT]` mode indicator; `service_deck_frame` uses `vinyl_speed` for display smoothing; `DeckRenderState.analysing` includes `vinyl_mode` to suppress overview bar markers on loaded decks; both `overview_empty` call sites pass `vinyl_mode`
- `src/render/mod.rs`: `overview_empty` takes `vinyl_mode: bool`; skips bar markers when true

## Intent

TJ's design philosophy is rooted in the craft of manual DJing — listening and adjusting by ear, as on turntables. The BPM infrastructure (auto-detection, beat grid, tick marks, beat jumps) is a useful aid but should not feel like the primary purpose of the application. Vinyl mode is a global toggle that hides all BPM-related machinery and replaces it with a simple percentage-based speed control and time-based navigation, giving the user nothing but playback and the pitch slider.

## Specification Deltas

### ADDED

**Vinyl mode toggle** — `` ` `` (backtick) globally toggles vinyl mode on and off. The mode applies to both decks simultaneously. `¬` (Shift+`` ` ``, UK keyboard) replaces `` ` `` as the terminal refresh binding.

**Playback speed display** — In vinyl mode, the BPM field in the info bar is replaced by a playback speed display showing the current speed as a percentage of the track's nominal rate (e.g. `+0.3%`, `-1.2%`, `0.0%`). The percentage is derived from a per-deck `vinyl_speed: f32` value (1.0 = nominal), which is passed directly to the audio player and is independent of BPM state. `vinyl_speed` resets to 1.0 whenever a new track is loaded in vinyl mode.

**Speed adjustment** — The existing per-deck BPM adjustment keys (`x`/`s` for Deck A, `v`/`f` for Deck B) remain active in vinyl mode but adjust `vinyl_speed` in 0.001 steps (±0.1%) rather than adjusting BPM. The updated `vinyl_speed` is applied to the player immediately via `set_speed`. The same keys therefore work identically in both modes; only the increment unit and display change.

**Time-based jumps** — In vinyl mode, the beat jump keys are remapped to fixed time intervals that match the beat jump sizes at 120 BPM:

| Keys | Beat mode | Vinyl mode |
|------|-----------|------------|
| `1` / `q` | ±1 beat | ±0.5 s |
| `2` / `w` | ±4 beats | ±2 s |
| `3` / `e` | ±16 beats | ±8 s |
| `4` / `r` | ±64 beats | ±32 s |

**Mode indicator** — A `[VINYL]` label is shown in the detail info bar (the shared row above both waveforms) whenever vinyl mode is active, alongside the existing zoom level. In beat mode this label is absent. Because the mode is global, a single indicator in the shared row is sufficient.

**Session persistence** — The active mode is stored in the cache file alongside `audio_latency_ms`. On startup the last-used mode is restored. The default for a fresh installation is beat mode.

**Mode transition: vinyl → beat** — When switching back to beat mode, `vinyl_speed` is converted into a BPM adjustment: `bpm = base_bpm × vinyl_speed`. The player speed is unchanged; the beat grid and display immediately reflect the new BPM. If `base_bpm` is not established, the conversion is skipped and `vinyl_speed` is discarded.

**Beat mode without established BPM** — When a deck in beat mode has no established BPM (no analysis result, no cached value), the BPM field in the info bar is replaced by the same percentage speed display used in vinyl mode (e.g. `+0.3%`, `0.0%`). Tick marks and phase offset remain present in the display — they are dormant rather than hidden. Once a BPM is established (via analysis, tap, or cache load) the display reverts to the normal BPM field. This is the key distinction from vinyl mode: vinyl mode actively hides ticks, offset, beat flash, and the metronome indicator; beat mode without BPM merely substitutes the display field while keeping all beat infrastructure visible.

**BPM display precision** — In beat mode, the BPM field is displayed rounded to one decimal place (e.g. `120.4` rather than `120.36`). The underlying value retains full precision; rounding is display-only. This prevents the display from showing values that cannot be reached via the 0.1 BPM adjustment steps.

**BPM analysis suppressed** — BPM analysis does not run while vinyl mode is active. If a track is loaded in vinyl mode, no analysis thread is started. If vinyl mode is toggled off for a deck whose BPM was not established before entering vinyl mode, the deck remains without an established BPM (the user may tap or use auto-detect as normal).

### MODIFIED

**Info bar** — In vinyl mode, the BPM field, beat flash, phase offset, and metronome indicator are replaced by the speed percentage display. The nudge mode indicator, level, latency, and spectrum strip are unchanged.

**Waveform** — Beat tick marks and the cue column are hidden in vinyl mode. The waveform itself is unchanged.

**Notification bar** — The rename offer and tag editor are unaffected. The BPM confirmation prompt does not appear in vinyl mode (analysis is suppressed).

**`SPEC/deck.md`** — Updated to document vinyl mode's effect on beat detection, display, and jump behaviour.

**`SPEC/render.md`** — Updated to document the info bar and waveform changes in vinyl mode.

**`SPEC/config.md`** — `` ` `` reassigned to vinyl mode toggle; `¬` added as terminal refresh.

**`SPEC/cache.md`** — Updated to document vinyl mode persistence alongside `audio_latency_ms`.

## Scope

- **In scope**: the global toggle; speed percentage display and 0.1% adjustment steps; time-based jumps; suppression of BPM analysis, tick marks, beat flash, offset display, and metronome indicator.
- **Out of scope**: per-deck vinyl mode; waveform colour changes; filter, level, cue, and browser behaviour (all unchanged).
