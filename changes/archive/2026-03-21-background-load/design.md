# Design: Background Track Loading
**Status: Approved**

## Approach

The key insight is that `load_deck()` does two things: (a) spawns a decode thread and (b) blocks the TUI polling that thread until done. This change keeps (a) and removes (b).

### `PendingLoad`

A new struct holds everything needed to track an in-progress decode:

```rust
struct PendingLoad {
    filename: String,
    path:     PathBuf,
    rx:       mpsc::Receiver<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>,
    decoded:  Arc<AtomicUsize>,
    total:    Arc<AtomicUsize>,
}
```

`filename` and the progress atomics are read by the render path each frame. `rx` is polled via `try_recv()` each frame; on success the full `Deck` is constructed from the received audio data (player, seek handle, waveform, BPM thread — unchanged from today) and moved into `decks[slot]`.

### Load initiation

A new `start_load()` function replaces `load_deck()`. It accepts a path and the mixer, spawns the decode thread, and returns a `PendingLoad`. No terminal interaction. Called from:

- `main()` when a CLI file arg is present, passing the `PendingLoad` into `tui_loop()`
- The `BrowserResult::Selected` handler inside `tui_loop()`

`tui_loop()` signature changes from `initial_deck: Option<Deck>` to `initial_load: Option<PendingLoad>`.

### State in `tui_loop`

```rust
let mut pending_loads: [Option<PendingLoad>; 2] = [initial_load, None];
```

The existing `decks: [Option<Deck>; 2]` initialises as `[None, None]`.

The `global_notification` "No track loaded" hint fires only when both `decks` and `pending_loads` are empty for slot 0.

### Per-frame polling

At the top of each frame (alongside the existing `service_deck_frame` calls), for each slot:

```
if pending_loads[slot].rx.try_recv() == Ok(audio_data):
    construct Deck from audio_data
    decks[slot] = Some(deck)
    pending_loads[slot] = None
```

Errors from `try_recv` go to a per-slot error notification displayed in the notification row.

### Rendering

Before the `terminal.draw` closure, compute a loading label per slot:

```rust
let loading_label: [Option<String>; 2] = std::array::from_fn(|slot| {
    let p = pending_loads[slot].as_ref()?;
    let done  = p.decoded.load(Ordering::Relaxed);
    let total = p.total.load(Ordering::Relaxed);
    let pct   = if total > 0 { format!(" {}%", (done * 100 / total).min(100)) } else { String::new() };
    Some(format!("Loading {}…{}", p.filename, pct))
});
```

In the draw closure, the deck-A and deck-B else branches (currently rendering `notification_line_empty()`) check `loading_label[slot]` and render the loading string in `DarkGray` if present.

The full-screen loading bar widget and its gauge are removed.

## Tasks

1. ✓ Impl: Add `PendingLoad` struct and `start_load()` function; remove `load_deck()`
2. ✓ Impl: Change `tui_loop()` signature to accept `Option<PendingLoad>`; initialise `pending_loads`
3. ✓ Impl: Update `main()` to call `start_load()` for CLI file arg instead of `load_deck()`
4. ✓ Impl: Replace `load_deck()` call in `BrowserResult::Selected` handler with `start_load()`
5. ✓ Impl: Per-frame polling — complete deck construction on receive, surface errors to notification row
6. ✓ Impl: Render loading label in notification rows; remove full-screen loading bar
7. ✓ Impl: Update `global_notification` hint condition to also check `pending_loads`
8. Process: Confirm ready to archive
