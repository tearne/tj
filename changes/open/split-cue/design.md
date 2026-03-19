# Design: Split Cue Mode
**Status: Draft**

## Approach

A single `Arc<AtomicBool>` (`split_cue_active`) is shared across both decks' audio sources and the UI/input layer. It is created once in `tui_loop` before any deck is loaded.

`FilterSource` gains two new fields — `split_cue_active` and `deck_slot` (0 = left, 1 = right) — and handles both concerns in `next()`:
- **Filter bypass**: when split cue is active, treat `filter_offset` as 0 (passthrough, IIR state preserved).
- **Channel masking**: zero the sample if the current channel index does not match the deck's assigned channel.

Level bypass is handled at toggle time in the event loop by calling `player.set_volume(1.0)` on both decks; restoring their stored `volume` when deactivated.

`load_deck` receives `Arc<AtomicBool>` and `deck_slot: usize` and forwards them into `FilterSource::new()`. Both are also stored in `DeckAudio` so the toggle handler can call `player.set_volume()`.

UI: the global status bar render checks `split_cue_active` and prepends a `[split cue]` label in amber when set.

## Tasks

1. ✓ Add `SplitCueToggle` to the `Action` enum and config keymap (`\`)
2. ✓ Extend `FilterSource` with `split_cue_active: Arc<AtomicBool>` and `deck_slot: usize`; implement bypass + masking in `next()`
3. ✓ Thread `split_cue_active` + `deck_slot` through `load_deck` → `FilterSource::new()`
4. ✓ Handle `SplitCueToggle` in the event loop: toggle atomic, adjust `player.set_volume()` on all loaded decks
5. ✓ Render `[split cue]` label in the global status bar
6. ✓ Bump version (patch)
