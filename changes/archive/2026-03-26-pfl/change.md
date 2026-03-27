# PFL Monitor
**Type**: Proposal
**Status**: Done

## Intent

When DJing with headphones and a single audio output, the DJ needs to monitor an incoming track privately before bringing it in. PFL (Pre-Fader, Pre-Filter Listen) routes a deck's raw signal to the left channel of the monitor output at an independently controlled level, leaving the right channel carrying the main mix at all times.

## Specification Deltas

### `SPEC/audio.md` — PFL Monitor (new)

- Each deck has a `pfl_level` with four steps: 0, 33%, 66%, 100%. Default is 0. Not persisted between sessions.
- PFL is exclusive: at most one deck can have a non-zero `pfl_level` at a time.
- The PFL signal is tapped pre-fader and pre-filter — the raw decoded audio, unaffected by `level` or `filter_offset`.
- Monitor output routing:
  - Right channel: always carries the main mix (both decks at their respective levels and filter settings).
  - Left channel: carries the active deck's PFL signal at `pfl_level` when any PFL is active; otherwise carries the main mix.
  - When PFL is active, the main mix is suppressed entirely on the left channel.

### `SPEC/keymap.md` — Per-Deck Controls (modified)

| Key | Action |
|-----|--------|
| `Space+s` | Deck 1 PFL up (0 → 33 → 66 → 100%) |
| `Space+x` | Deck 1 PFL down / cancel active PFL |
| `Space+f` | Deck 2 PFL up (0 → 33 → 66 → 100%) |
| `Space+v` | Deck 2 PFL down / cancel active PFL |

Pressing PFL up on a deck while the other deck's PFL is active cancels the other deck and starts the pressed deck at 33%. Pressing PFL down on a deck while the other deck's PFL is active cancels the other deck (the pressed deck remains at 0).

### `SPEC/render.md` — Deck Info Bar (modified)

Each deck displays a PFL level indicator alongside the main level indicator, rendered in a distinct colour (e.g. cyan). The indicator is only visible when `pfl_level > 0`.

## Design

`FilterSource` gains a `pfl_level: Arc<AtomicU8>` (values 0, 33, 66, 100). A shared `pfl_active_deck: Arc<AtomicUsize>` (sentinel value for "none") coordinates exclusivity across both sources and the draw loop.

Monitor output is a stereo mix assembled in `FilterSource::next()`:
- Right sample: normal signal path (level + filter applied).
- Left sample: when this deck holds PFL, the pre-filter/pre-fader sample scaled by `pfl_level`; otherwise the normal signal path.

PFL key handlers in the input loop update `pfl_level` and `pfl_active_deck` atomically, enforcing exclusivity and the 33% start-on-switch rule.

`build_deck` receives `pfl_active_deck` (shared) and `pfl_level` (per-deck) and forwards them into `FilterSource::new()`.

## Log

- Added `pfl_level: Arc<AtomicU8>`, `pfl_active_deck: Arc<AtomicUsize>`, `deck_slot`, and `deck_volume: Arc<AtomicU32>` to `FilterSource`; routing logic in `next()` zeroes the left channel for the non-PFL deck and emits raw*scale on the left + filtered*vol on the right for the PFL deck
- Added `pfl_level` and `deck_volume_atomic` to `DeckAudio`; `pfl_level: u8` to `Deck`
- Added `Deck1PflUp/Down`, `Deck2PflUp/Down` actions; `Space+s/x/f/v` bindings in `config.toml`
- Level handlers updated to sync `deck_volume_atomic` and skip `player.set_volume()` when PFL is active for that deck
- Cyan PFL bar indicator in `info_line_for_deck` (visible only when pfl_level > 0)
- Simplified to toggle-only (Space+x / Space+v); removed step levels and up/down bindings; `Deck1PflToggle` / `Deck2PflToggle` replace the four previous actions
- `SPEC/audio.md` PFL section added; `SPEC/config.md` key table and keyboard diagram updated

## Scope

- **In scope**: single-output PFL monitor as described.
- **Out of scope**: master/house output (future); per-deck headphone mix blend; more than two decks (exclusivity model scales naturally but key assignments are unspecified).
