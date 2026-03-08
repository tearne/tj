# Design: Switchable Detail Waveform Render Mode
**Status: Ready for Review**

## Approach

Two modes share the same `BrailleBuffer` struct and the same UI-thread viewport logic; only the background thread's recompute strategy differs.

### Buffer mode (current behaviour)
- `buf_cols = 3 × cols`; recomputes only on zoom/resize/drift ≥ cols.
- Reads raw audio position (`pos_bg`).
- Background thread sleeps 8 ms between cycles.

### Live mode (new)
- `buf_cols = 1 × cols`; recomputes every cycle.
- Reads the **smooth display position** via a new shared atomic (`display_pos_shared`, written by the UI thread each frame in units of interleaved samples — same units as `seek_handle.position`).
- Background thread sleeps 4 ms between cycles (fast enough that the anchor is at most ~180 samples stale at 44 100 Hz — well under one braille column at any zoom level, so `viewport_start` stays 0).
- If `viewport_start` is out of range (seek in progress, first frame), blank rows are shown as before.

### New shared state

```rust
let live_mode      = Arc::new(AtomicBool::new(false));
let display_pos_shared = Arc::new(AtomicUsize::new(0));
```

UI thread writes `display_pos_shared` each frame:
```rust
display_pos_shared.store(
    (smooth_display_samp as usize) * seek_handle.channels as usize,
    Ordering::Relaxed,
);
```

### Background thread changes

```rust
let live = live_mode_bg.load(Ordering::Relaxed);
let pos_samp = if live {
    display_pos_bg.load(Ordering::Relaxed) / ch_bg as usize
} else {
    pos_bg.load(Ordering::Relaxed) / ch_bg as usize
};
let buf_cols = if live { cols } else { cols * 3 };
let must_recompute = live
    || cols != last_cols || rows != last_rows || zoom != last_zoom || drift_cols >= cols;
let sleep_ms: u64 = if live { 4 } else { 8 };
```

### Key binding and UI

`m` toggles `live_mode` (local `bool`; stored to `live_mode_shared` each frame alongside `display_pos_shared`).

Mode shown in the key hints line, e.g. `m:buf` / `m:live`.

## Tasks

1. ✓ Impl: Add `live_mode` and `display_pos_shared` atomics; wire up `m` key toggle; write `display_pos_shared` each frame; pass both arcs into the background thread.
2. ✓ Impl: Update background thread — read `live` flag, select position source, set `buf_cols` and `must_recompute` accordingly; update sleep duration.
3. ✓ Impl: Update key hints to show current mode.
4. ✓ Verify: `cargo build`; manual test — both modes scroll smoothly; toggling with `m` switches behaviour visibly; ticks stable in both modes.
5. ✓ Process: Archive `render-mode`; update SPEC.md.
