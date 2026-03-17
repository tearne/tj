# Proposal: Remove Deck Visibility Legacy Code
**Status: Draft**

## Problem

Two variables established when deck visibility was optional are now constants that never change:

- `active_deck: usize = 0` — never reassigned. All derived values are compile-time constants: `inactive = 1`, `a_is_active = true`, `b_is_active = false`.
- `deck_visible: [bool; 2] = [true, true]` — never reassigned.

This produces dead code in several places:

1. **Two layout match blocks** (empty-deck path and main path), each with three arms. Only `[true, true]` ever matches; the `[true, false]` and `[false, true]` arms are unreachable (~30 lines each).
2. **Conditional guards** `if deck_visible[0]`, `if deck_visible[1]`, `if deck_visible[inactive]` — always true; their bodies always execute.
3. **Derived booleans** `a_is_active = active_deck == 0` (always `true`) and `b_is_active = active_deck == 1` (always `false`) used to branch render and notification logic.

## Change

- Remove `deck_visible`. Inline the `[true, true]` layout directly; delete the other match arms. Remove the `if deck_visible[...]` guards.
- Remove `active_deck`. Replace with literal `0` where it indexes `decks`, and literal `1` for `inactive`. Remove `a_is_active` / `b_is_active` and inline their values (`true` / `false`) or simplify the branches they gate.
- The empty-deck handler comment `active_deck == 0` check (line ~613) simplifies to unconditional `empty_is_a = true`.
- `store_speed_ratio(active_deck, ...)` becomes `store_speed_ratio(0, ...)`.
- `let active_buf = if active_deck == 0 { &buf_a } else { &buf_b }` becomes `let active_buf = &buf_a`.

## Scope

Code deletion only — no behaviour change. The two-deck layout is unchanged; only the branching infrastructure for layouts that can no longer occur is removed.
