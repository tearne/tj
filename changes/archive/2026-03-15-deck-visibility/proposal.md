# Proposal: Deck Visibility
**Status: Complete**

## Intent

The user can hide individual deck sections from the layout to focus on a single deck. At least one deck must always remain visible. This supersedes the earlier `single-deck-layout` proposal, which described a one-way startup default; the behaviour here is fully reversible and user-driven.

## Specification Deltas

### ADDED

- **Deck visibility**: each deck has an independent visible/hidden state. A hidden deck's sections (detail waveform, notification bar, info bar, overview) are removed from the layout entirely; the remaining deck expands to fill the freed space.
- **Disable bindings**: `G` disables Deck A; `H` disables Deck B. Disabling is only permitted when the deck is paused and the other deck is currently visible; the action is a no-op otherwise.

- **Single-deck layout**: when one deck is hidden, the layout is:

  ```
  ┌─ tj ──────────────────────────────────────────────────────┐
  │  Detail info bar         (Constraint::Length(1))           │
  │  Detail waveform         (Constraint::Min(detail_height))  │
  │  Notification bar        (Constraint::Length(1))           │
  │  Info bar                (Constraint::Length(1))           │
  │  Overview                (Constraint::Length(4))           │
  │  Global status bar       (Constraint::Length(1))           │
  └────────────────────────────────────────────────────────────┘
  ```

  The detail waveform expands to fill all remaining vertical space.

### MODIFIED

- **`g` / `h` bindings**: unchanged — select (activate) Deck A / Deck B respectively. Selecting a hidden deck makes it visible again.
- **Deck selection**: if the active deck is disabled, the active deck switches to the remaining visible deck automatically.
- **State preservation**: disabling a deck is a UI-only change. All deck state (track, position, BPM, offset, volume, filter, etc.) is preserved in memory. Re-activating the deck with `g`/`h` restores the full view with all previous settings intact.
