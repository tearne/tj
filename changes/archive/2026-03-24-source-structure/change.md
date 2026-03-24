# Source Structure
**Type**: Proposal
**Status**: Approved

## Intent

`src/main.rs` is approaching 5 000 lines. All domain logic — audio decoding, playback, tag handling, BPM cache, waveform rendering, the file browser, key bindings — lives in a single flat file. This makes navigation slow, obscures ownership boundaries, and will compound as the application grows.

The goal is to decompose `main.rs` into focused modules, each owning a clearly named domain concept, without changing any observable behaviour. The `SPEC/` directory is reorganised to mirror the module structure, with one file per module alongside the three cross-cutting files that remain.

## Specification Deltas

### ADDED

- Source code is organised into modules named after domain concepts:

  | Module    | Owns |
  |-----------|------|
  | `audio`   | Source chain (`TrackingSource`, `FilterSource`, `SeekHandle`), decode, `scrub_audio`, `play_click_tone` |
  | `deck`    | `Deck`, `DeckAudio`, `TempoState`, `TapState`, `DisplayState`, `SpectrumState`, `NudgeMode`, `Notification`, `TagEditorState`; tap BPM computation, `anchor_beat_grid_to_cue`, `apply_offset_step` |
  | `render`  | All TUI rendering functions, `BrailleBuffer`, `SharedDetailRenderer`, braille helpers, bar-tick geometry |
  | `browser` | `BrowserState`, `BrowserEntry`, `EntryKind`, `BrowserResult`, `run_browser`, `is_audio` |
  | `tags`    | `stem_conforms`, `collect_tags`, `read_tags_for_editor`, `propose_rename_stem`, `sanitise_for_filename`, `read_track_name` |
  | `cache`   | `Cache`, `CacheEntry`, `CacheFile`, `hash_mono`, `detect_bpm` |
  | `config`  | `Action`, `KeyBinding`, `DisplayConfig`, `load_config`, key parsing |

  `main.rs` retains only: `main`, `tui_loop`, `service_deck_frame`, `start_load`, `build_deck`, `PendingLoad`.

- No `utils`, `helpers`, or other catch-all modules are introduced.

- `SPEC/` is reorganised to mirror the module structure. Each module has a corresponding spec file:

  | `SPEC/` file   | Source |
  |----------------|--------|
  | `audio.md`     | `SPEC/audio.md` — unchanged |
  | `deck.md`      | `SPEC/transport.md` — retitled "Deck" |
  | `render.md`    | `SPEC/layout.md` + `SPEC/waveforms.md` — merged and expanded (see below) |
  | `browser.md`   | File browser section extracted from `SPEC/overview.md` |
  | `tags.md`      | `SPEC/tags.md` — unchanged |
  | `cache.md`     | Caching section extracted from `SPEC/architecture.md` |
  | `config.md`    | `SPEC/keymap.md` — retitled "Config" |

  Cross-cutting files retained: `overview.md` (launching and constraints only), `architecture.md` (threading only), `verification.md`.

- `SPEC/render.md` expands the waveform rendering pipeline documentation to close known gaps in the existing `waveforms.md`. The following are fully specified:
  - The exact transformation applied to tick values through the half-column shift: how `0x47` (left half-column tick) becomes `0xB8` (right half-column tick) and vice versa, and why those byte values encode the correct braille dot patterns
  - The draw thread's tick rendering algorithm, including behaviour when a tick coincides with the playhead column
  - The cue mark rendering: character, colour, and precedence rules when a cue column coincides with a tick mark or the playhead
  - Column coincidence rules in general: the priority order when waveform, tick, cue, and playhead occupy the same screen column
  - Derivation of the `0x47` and `0xB8` sentinel values from the braille encoding

### MODIFIED

- No user-visible behaviour changes. The refactor is purely structural.

## Scope

- **In scope**: decomposing `main.rs` into the modules above; adjusting visibility (`pub`, `pub(crate)`) at boundaries as required by the compiler; reorganising `SPEC/` to mirror the module structure as described.
- **Out of scope**: renaming types or functions; changing logic; introducing new abstractions or traits. Any such changes are deferred to follow-on proposals.

## Log

All 7 modules created at `src/{audio,browser,cache,config,deck,render,tags}/mod.rs`. `main.rs` rewritten to retain only `main`, `tui_loop`, `service_deck_frame`, `start_load`, `build_deck`, `PendingLoad`, plus module declarations and `use` imports. Builds cleanly with zero warnings. All 9 tests pass. `SPEC/` reorganised to mirror module structure: `transport.md` → `deck.md`, `keymap.md` → `config.md`, `layout.md` + `waveforms.md` merged into `render.md` with expanded pipeline documentation, `browser.md` and `cache.md` extracted from `overview.md` and `architecture.md` respectively. Pre-existing metronome key inconsistency in `config.md` (lists `'`, code uses `B`/`N`) noted for a separate fix.
