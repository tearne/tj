# Design: Deck Visibility
**Status: Complete**

## Approach

### State

Add `deck_visible: [bool; 2]` to `tui_loop`, initialised to `[true, true]`.
This is the only new persistent state required; all deck audio/BPM/display
state remains in the existing `deck` / `decks` slots.

### Actions and bindings

Add two `Action` variants:

```rust
DeckAHide, DeckBHide,
```

Register them in `ACTION_NAMES` as `"deck_a_hide"` / `"deck_b_hide"` and add
default bindings `G` / `H` to `resources/config.toml`.

### Handler logic

**`DeckAHide` / `DeckBHide`** (processed as a global action before the
active-deck branch):

1. Identify `hide_slot` (0 or 1) and `other_slot` (1 or 0).
2. Guard: no-op unless the deck to hide is paused **and** `deck_visible[other_slot]`.
3. If `hide_slot == active_deck`: store `deck` back to `decks[active_deck]`,
   switch `active_deck = other_slot`, `continue` (triggers deck-load at top
   of loop).
4. Set `deck_visible[hide_slot] = false`.

**`DeckSelectA` / `DeckSelectB`** — extend existing handlers:

```rust
// before the existing active_deck switch
deck_visible[selected_slot] = true;
```

If the selected deck is already visible this is a no-op. If it was hidden,
it becomes visible again with all state intact (no reload needed).

**Auto-switch on hide**: handled in step 3 above — if the active deck is the
one being hidden, we pivot to the other deck in the same action, so the
invariant "active deck is always visible" is maintained without a separate
polling step.

### Layout branching

After the current `terminal.draw` closure computes `inner`, branch on
`deck_visible`:

**Both visible** — existing 10-constraint layout (indices unchanged).

**Only A visible** (`deck_visible == [true, false]`):

```
chunks[0]  Length(1)              detail info bar
chunks[1]  Min(a_detail_h)        detail A  ← expands to fill
chunks[2]  Length(1)              notif A
chunks[3]  Length(1)              info A
chunks[4]  Length(4)              overview A
chunks[5]  Length(1)              global bar
chunks[6]  Min(0)                 spacer
```

**Only B visible** (`deck_visible == [false, true]`):

```
chunks[0]  Length(1)              detail info bar
chunks[1]  Min(b_detail_h)        detail B  ← expands to fill
chunks[2]  Length(1)              notif B
chunks[3]  Length(1)              info B
chunks[4]  Length(4)              overview B
chunks[5]  Length(1)              global bar
chunks[6]  Min(0)                 spacer
```

Each branch renders only the visible deck's widgets. After `Layout::split`,
store `chunks[1].height` into `shared_renderer.rows` so the background thread
knows the expanded detail height.

The draw code already isolates Deck A and Deck B rendering into separate
blocks; the layout branch restructures only the constraint list and chunk
index mapping — the render logic for each deck is otherwise unchanged.

## Tasks

1. ✓ Config: add `deck_a_hide = "G"` and `deck_b_hide = "H"` to `resources/config.toml`
2. ✓ Impl: add `Action::DeckAHide` / `Action::DeckBHide` variants and `ACTION_NAMES` entries
3. ✓ Impl: add `deck_visible` state; hide handlers; `DeckSelectA`/`B` un-hide; layout branching
4. ✓ Verify: build and smoke-test hide/show/auto-switch/single-deck layout expansion
5. ✓ Process: archive
