# Design: Remove Deck Visibility Legacy Code
**Status: Complete**

## Changes

- Removed `active_deck: usize = 0` and `deck_visible: [bool; 2] = [true, true]` declarations.
- `inactive` is now `let inactive = 1usize;` — constant, no arithmetic.
- Both layout `match deck_visible` blocks replaced with the inlined `[true, true]` layout directly.
- `if deck_visible[inactive]` guard removed; body always executes.
- `if deck_visible[0]` / `if deck_visible[1]` guards removed.
- `a_is_active` / `b_is_active` removed. Deck A render section simplified to always use `deck` (the main handler deck). Deck B render section simplified to always use `decks[1]` (the inactive deck), restructured as `if let Some(ref d) = decks[1] { ... } else { placeholder }`.
- `active_buf = if active_deck == 0 { &buf_a } else { &buf_b }` → `&buf_a`.
- `if active_deck == 0 { store pos_a } else { store pos_b }` → `store pos_a` directly.
- `store_speed_ratio(active_deck, ...)` → `store_speed_ratio(0, ...)`.
- `decks[active_deck] = Some(deck)` → `decks[0] = Some(deck)` (all occurrences).
- `decks[active_deck].take()` → `decks[0].take()`.
- Empty-deck path: `empty_is_a` and related branches removed; area names inlined directly.
- `active_detail_area = if a_is_active { ... } else { ... }` → `area_detail_a`.
