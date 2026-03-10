# Design: Dedicated Beat Jump Buttons
**Status: Approved**

## Approach

Remove the `BEAT_UNITS` constant, `DEFAULT_BEAT_UNIT_IDX`, and `beat_unit_idx` state variable. Replace the two `[`/`]` key handlers and `'1'..='7'` unit-selector handler with eight individual key handlers, each encoding the beat count directly. The jump arithmetic is unchanged: `N * 60.0 / bpm` seconds.

The info bar currently shows `×{beat_unit}` — remove that field. No other UI changes are needed.

Key assignments are temporary hard-coded bindings that will be replaced by the keyboard-mapping change:

| Key | Action | Beats |
|-----|--------|-------|
| `1` | `jump_forward_1` | +1 |
| `q` | `jump_backward_1` | −1 |
| `2` | `jump_forward_4` | +4 |
| `w` | `jump_backward_4` | −4 |
| `3` | `jump_forward_16` | +16 |
| `e` | `jump_backward_16` | −16 |
| `4` | `jump_forward_64` | +64 |
| `r` | `jump_backward_64` | −64 |

A helper closure/function `do_jump(beats: i32)` avoids repeating the seek logic eight times:

```rust
let do_jump = |beats: i32| {
    let jump = beats.abs() as f64 * 60.0 / bpm as f64;
    if beats < 0 {
        let target = (seek_handle.current_pos().as_secs_f64() - jump).max(0.0);
        if player.is_paused() { seek_handle.seek_direct(target); }
        else { seek_handle.seek_to(target); }
    } else {
        let target = seek_handle.current_pos().as_secs_f64() + jump;
        if target < total_duration.as_secs_f64() {
            if player.is_paused() { seek_handle.seek_direct(target); }
            else { seek_handle.seek_to(target); }
        }
    }
};
```

Then each handler is just `do_jump(1)`, `do_jump(-1)`, etc.

## Tasks

1. ✓ **Impl**: Remove `BEAT_UNITS`, `DEFAULT_BEAT_UNIT_IDX`, `beat_unit_idx`; remove `×{beat_unit}` from info bar format string
2. ✓ **Impl**: Remove `'1'..='7'` unit-selector handler and `[`/`]` jump handlers; add `do_jump` helper and 8 key handlers
3. ✓ **Verify**: Confirm each of the 8 keys jumps by the correct number of beats; confirm info bar no longer shows beat unit
4. ✓ **Process**: Confirm ready to archive
