# Deck 3

## Intent
The player supports two decks. This change adds a third, enabling three-way mixes.

The selected deck model introduced in the keyboard layout change accommodates the third deck without additional key assignments: `Space+3` selects it, and all selected-deck controls apply to it immediately. The mixer section gains a third column for level, gain, and filter.

## Approach

The `decks` array grows from two to three `Option<Deck>` slots. `selected_deck` extends to cover index 2. `Space+3` maps to a new `SelectDeck3` action. The mixer gains a third column bound to keys `9` / `O` (filter HPF/LPF), `L` (level/gain+), `.` (level/gain−), following the same pattern as columns `7`/`U`/`J`/`M` and `8`/`I`/`K`/`,`. New `Deck3*` mixer action variants are added for level, gain, filter, and filter slope. `pfl_active_deck` already stores a slot index so three-deck PFL mutual exclusion requires no structural change.

**Deck reorder** — `Space+=` swaps the deck in slot 0 with slot 1; `Space+-` swaps slot 1 with slot 2. The swap exchanges the entire `Option<Deck>` value (all state: track, transport, BPM, pitch, mixer, cue). The renderer's per-slot data (`waveform`, `seek_handle`, channels, sample rate) must be swapped in the same operation to keep rendering consistent. If `selected_deck` equals one of the two swapped indices it is updated to the other, so the operator continues controlling the same physical content after the swap.

**UI layout** — The detail section gains a third waveform panel below deck 2. Between the deck 2 and deck 3 waveform panels a single shared beat-marker row is inserted; it shows the OR-combined full-height tick marks of both decks, making beat phase alignment directly visible. The existing tick rows within the deck 1 and deck 2 waveform blocks are unchanged. The overview, info bar, and notification bar sections each gain a third instance for deck 3, extending the existing per-deck layout pattern. The empty deck panel label for deck 3 is `"C"`. The config keyboard diagram and `SPEC/config.md` are updated with the third mixer column and the `Space+=`/`Space+-` deck swap bindings. `SharedRenderer` is extended from two to three slots. `service_deck_frame` is extended to iterate over slot 2.

**Beat marker row encoding** — The existing `0x47`/`0xB8` full-height tick encoding is reused unchanged. The background thread computes tick positions for both D2 and D3 and OR-combines them into a single tick array for the shared row. No new sentinel values and no extension to the half-column shift transform are required.

**PFL with three decks** — The existing handlers use `let other = 1 - selected_deck`, which underflows as `usize` when `selected_deck == 2`. All PFL action handlers that currently clear the "other" deck must be updated to iterate over all slots except `selected_deck` when clearing PFL state.

Review cadence: at the end.

## Plan

- [ ] UPDATE SPEC — `SPEC/config.md`: add `Space+3` to deck controls section; add third mixer column (`9`/`O`/`L`/`.`) to mixer table; add `Space+=`/`Space+-` swap bindings to global section; update legend
- [ ] UPDATE SPEC — `SPEC/deck.md`: extend deck selection section to cover three decks and `Space+3`
- [ ] UPDATE SPEC — `SPEC/render.md`: update UI layout structure to add deck 3 waveform, info bar, notification bar, and overview panels; add shared beat-marker row between D2 and D3 panels; extend empty deck panel label list to include `"C"` for deck 3
- [ ] UPDATE IMPL — Extend `decks` from `[Option<Deck>; 2]` to `[Option<Deck>; 3]` in `main.rs`; extend `service_deck_frame` loop to cover slot 2
- [ ] ADD IMPL — Add `SelectDeck3` action variant; bind to `Space+3` in config
- [ ] ADD IMPL — Add `Deck3*` mixer action variants (level up/down/max/min, gain increase/decrease, filter increase/decrease/reset, filter slope increase/decrease); bind to `9`/`O`/`L`/`.` per the mixer column pattern
- [ ] ADD IMPL — Add `SwapDeck1Deck2` and `SwapDeck2Deck3` action variants; bind to `Space+=` and `Space+-`; implement swap: exchange `decks[0]`/`decks[1]` or `decks[1]`/`decks[2]`, swap corresponding `SharedRenderer` slots, update `selected_deck` if it equals either swapped index
- [ ] UPDATE IMPL — PFL handlers (`PflOnOff`, `PflLevelUp`): replace `let other = 1 - selected_deck` with iteration over all slots except `selected_deck` when clearing PFL state
- [ ] UPDATE IMPL — Extend `SharedRenderer` from two to three slots
- [ ] UPDATE RENDER — Add third waveform panel to the detail section layout; insert shared beat-marker row between D2 and D3 panels (OR-combined tick arrays from both decks); add third overview, info bar, and notification bar panels
- [ ] UPDATE IMPL — Update embedded default `config.toml` with all new bindings (deck 3 mixer, `Space+3`, `Space+=`, `Space+-`)
- [ ] ADD IMPL — Build release binary (`cargo build --release`)
- [ ] TEST — Verify deck 3 selection and all selected-deck controls; deck 3 mixer (level, gain, filter, PFL); deck swap in all combinations including selected-deck tracking; PFL mutual exclusion across all three decks; shared beat-marker row visibility; UI layout with all three decks loaded and empty

## Conclusion
