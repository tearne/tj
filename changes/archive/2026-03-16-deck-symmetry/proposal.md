# Proposal: Deck Symmetry Refactor
**Status: Draft**

## Problem

The main loop bifurcates on deck 1's state:

```
let Some(mut deck) = decks[0].take() else {
    // "empty-deck branch": reduced handler, deck 2 actions manually replicated
    continue;
};
// "main branch": full handler, deck 1 as THE deck
```

Everything in the main branch treats deck 1 as the primary deck. Deck 2 is always `decks[1]` — an afterthought accessed conditionally. This creates structural asymmetry that causes repeated bugs:

- Actions only work when deck 1 is loaded (latency, zoom, display settings were all broken)
- Deck 2 metronome never fires — the beat-index computation and click-trigger are only coded for deck 1
- Deck 2 tap timeout never fires — the per-frame tap session check is deck 1 only
- Every new feature must be added to both branches separately, and the empty-deck branch routinely drifts behind
- The empty-deck branch has a separate render closure, a separate event loop, and a separate action match — all of which are partial copies of the main branch

The root cause: the original single-deck design used `decks[0].take()` to pull the active deck out for the frame. Multi-deck support was added on top without restructuring, creating a permanent primary/secondary hierarchy.

## Goal

A single code path that services both decks identically regardless of which slots are loaded. Adding or fixing any feature touches one place, not two.

## Proposed Structure

### Phase 1: Per-frame deck servicing (symmetric)

Extract a `service_deck(slot, decks, mixer, ...)` function, called for both slots before rendering:

```rust
for slot in 0..2 {
    service_deck(slot, &mut decks, &mixer, frame_count, &shared_renderer, &mut cache);
}
```

`service_deck` handles everything that currently only runs for deck 1 in the main branch:
- Poll `bpm_rx` for detection results
- Advance `smooth_display_samp` (wall-clock interpolation + drift correction)
- Compute beat index and fire metronome click if mode is on and beat changed
- Check tap session timeout and finalise tap BPM
- Update spectrum analyser chars

### Phase 2: Render (symmetric)

A single `terminal.draw()` closure, always rendering both slots. Each slot gets a `render_deck_panel(frame, slot, decks, ...)` call. If the slot is empty, renders placeholder. No branching on deck 1 state.

The detail info bar (zoom/latency) renders once, unconditionally.

### Phase 3: Event handling (single action match)

One event loop, one action match. Deck-specific actions dispatch to a helper:

```rust
fn apply_deck_action(action: DeckAction, slot: usize, decks: &mut [Option<Deck>; 2], ...) { ... }
```

Global actions (latency, zoom, waveform style, palette, help, quit) handled once.

### The `take()` problem

The current `decks[0].take()` was needed because `deck` is mutably borrowed across the whole frame body. With the new structure, `take()` can be scoped to the `service_deck` call:

```rust
if let Some(mut d) = decks[slot].take() {
    // service d
    decks[slot] = Some(d);
}
```

Rendering and event handling access decks by index without needing to hold a borrow across the frame.

## What Changes

| Current | New |
|---|---|
| Two render closures (empty-deck + main) | One render closure |
| Two event loops | One event loop |
| Two action match blocks (~400 lines each) | One action match |
| Per-frame BPM/metronome/tap only for deck 1 | Per-frame servicing for both slots |
| Deck 2 metronome is a no-op | Deck 2 metronome fires correctly |
| Deck 2 tap timeout never fires | Tap timeout fires for both decks |
| Display settings (zoom, height, style) unavailable when deck 1 empty | Always available |
| New features must be added twice | Added once |

## What Does Not Change

- Data structures (`Deck`, `TempoState`, `TapState`, `AudioPipeline`, etc.)
- Audio pipeline and playback
- Render helper functions (`info_line_for_deck`, `overview_for_deck`, etc.)
- Action enum and key bindings
- Cache and config

## Implementation Order

1. Extract `service_deck()` function — run for both slots each frame
2. Collapse the two render closures into one
3. Collapse the two action match blocks into one
4. Verify deck 2 metronome fires (previously broken)
5. Verify deck 2 tap timeout fires (previously broken)
6. Verify all display settings work with deck 1 empty
7. Delete the empty-deck branch entirely

## Risk

Medium. The logic is not changing — only where it lives. The main risk is missing a subtle interaction that relied on the bifurcated structure. Full manual test of both decks independently and together after implementation.
