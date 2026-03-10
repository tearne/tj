# Design: Micro-Jump
**Status: Approved**

## Approach

Add `c`/`d` key handlers that seek the playhead by ±5ms on `Press` and `Repeat` (key-repeat while held).

The fade-based `seek_to` is unsuitable here — the ~6ms fade window is longer than the 5ms jump itself, causing audible artefacts. Use `seek_direct` in all cases (playing or paused). At normal playback the discontinuity is inaudible at 5ms; if it proves problematic a fade can be introduced later.

Jump logic:
```rust
// forward
let target = (seek_handle.current_pos().as_secs_f64() + 0.005)
    .min(total_duration.as_secs_f64());
seek_handle.seek_direct(target);

// backward
let target = (seek_handle.current_pos().as_secs_f64() - 0.005).max(0.0);
seek_handle.seek_direct(target);
```

## Tasks

1. **Impl**: Add `c` (backward) and `d` (forward) handlers on `Press | Repeat` using `seek_direct` ±5ms; update help popup
2. **Verify**: Confirm single press moves 5ms; confirm holding key repeats continuously; confirm clamping at start/end
3. **Process**: Confirm ready to archive
