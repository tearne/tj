# Source Structure
**Type**: Proposal
**Status**: Draft

## Intent

`src/main.rs` is approaching 5 000 lines. All domain logic — audio decoding, playback, tag handling, BPM cache, waveform rendering, the file browser, key bindings — lives in a single flat file. This makes navigation slow, obscures ownership boundaries, and will compound as the application grows.

The goal is to decompose `main.rs` into focused modules, each owning a clearly named domain concept, without changing any observable behaviour. Each module directory receives a `SPEC.md` migrated from (or extracted from) the corresponding root `SPEC/` file, co-locating specification and implementation.

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

- Each module directory contains a `SPEC.md` co-located with its implementation. The root `SPEC/` files are migrated as follows:

  | Module `SPEC.md` | Source |
  |------------------|--------|
  | `src/audio/`     | `SPEC/audio.md` — moved as-is |
  | `src/deck/`      | `SPEC/transport.md` — moved and retitled "Deck" |
  | `src/render/`    | `SPEC/layout.md` + `SPEC/waveforms.md` — merged |
  | `src/browser/`   | File browser section extracted from `SPEC/overview.md` |
  | `src/tags/`      | `SPEC/tags.md` — moved as-is |
  | `src/cache/`     | Caching section extracted from `SPEC/architecture.md` |
  | `src/config/`    | `SPEC/keymap.md` — moved and retitled "Config" |

- Root `SPEC/` retains only cross-cutting files: `overview.md` (launching and constraints, with file browser section removed), `architecture.md` (threading section only), and `verification.md`.

### MODIFIED

- No user-visible behaviour changes. The refactor is purely structural.

## Scope

- **In scope**: decomposing `main.rs` into the modules above; adjusting visibility (`pub`, `pub(crate)`) at boundaries as required by the compiler; migrating and extracting `SPEC/` files into module directories as described.
- **Out of scope**: renaming types or functions; changing logic; introducing new abstractions or traits. Any such changes are deferred to follow-on proposals.
