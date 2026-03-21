# Proposal: Background Track Loading
**Status: Approved**

## Overview

When a track is selected in the browser, the player immediately returns to the deck view. The target deck is cleared to zero state and its notification bar shows loading progress. Loading continues in the background; the deck becomes active when ready.

## Current Behaviour

Loading a track always blocks the TUI with a full-screen loading bar:
- At startup with a CLI file arg: `load_deck()` runs before `tui_loop()` starts
- In-player via the browser: `load_deck()` is called on `BrowserResult::Selected`, suspending the TUI loop

In both cases the screen is replaced with a progress bar and the player is unresponsive for the duration.

## Proposed Behaviour

Loading is always non-blocking. The TUI starts immediately and all loads — including the startup CLI file arg — go through the same background path:

1. A load is initiated (startup arg or browser selection) → TUI is running, target deck slot is `None`
2. The target deck's notification row shows `Loading <filename>… <pct>%`
3. On completion the deck is constructed and inserted; the notification clears

The other deck (if loaded) continues playing and receiving input normally throughout. The full-screen loading bar is removed entirely.

## Design

### Loading state

Introduce a parallel structure alongside `decks: [Option<Deck>; 2]`:

```rust
pending_loads: [Option<PendingLoad>; 2]
```

```rust
struct PendingLoad {
    filename:     String,
    rx:           mpsc::Receiver<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>,
    decoded:      Arc<AtomicUsize>,
    estimated:    Arc<AtomicUsize>,
}
```

### Initiating a load

The same function handles both cases. At startup, `tui_loop()` accepts an `Option<PathBuf>` instead of an `Option<Deck>`; on browser selection, `BrowserResult::Selected(path)` triggers the same path:

- Stop and drop the outgoing deck (if any)
- Set `decks[target] = None`
- Spawn the decode thread (same logic as the body of current `load_deck()`, minus the blocking render loop)
- Store a `PendingLoad` in `pending_loads[target]`

`load_deck()` is removed.

### Per-frame polling (`service_deck_frame` or inline in the main loop)

Each frame, check `pending_loads[slot]`:
- If `rx.try_recv()` returns `Ok(result)` → construct the `Deck` (player, seek handle, waveform, BPM thread) and move it into `decks[slot]`; clear `pending_loads[slot]`
- Otherwise read progress from `decoded`/`estimated` atomics and store on `PendingLoad` for rendering

### Rendering

The notification rows (A/B) are currently only rendered when the deck is `Some`. Change to render the notification row unconditionally per slot:

- Deck `Some` → existing `notification_line_for_deck()` logic
- Deck `None` + `PendingLoad` → `Loading <filename>… <pct>%` in `DarkGray`
- Deck `None`, no pending load → empty row (existing `notification_line_empty()`)

The info and overview rows remain gated on the deck being `Some` (they render empty as today).

## Constraints

- The other deck must remain fully responsive (playback, input, rendering) during a load
- If the user opens the browser again for the same slot while a load is in progress, the pending load is cancelled (channel dropped) and a new one begins
- Decode errors surface in the notification row rather than via `eprintln!` + exit
- `load_deck()` is removed; there is one load path for all cases

## Verification

- Select a track for deck A while deck B is playing — deck B plays uninterrupted
- Notification row for the loading deck shows filename and percentage
- On completion, deck appears in zero state (paused, at position 0)
- Selecting a second track for the same slot before the first finishes cancels the first load cleanly
- Selecting a track when the terminal is small enough to hide the notification row does not crash
