# Info Line: Level Saturation + Tap Flash
**Type**: Fix
**Status**: Approved

## Changes

### 1. Level indicator — stepped yellow saturation

The level bar character is currently styled with a fixed `Rgb(120, 100, 0)` regardless
of the volume step. It should ramp from a dim yellow at the bottom step to the same
colour the spectrum analyser uses for active dots (`Color::Yellow` fg /
`Rgb(40, 33, 0)` bg) at the top step, so the two elements feel visually consistent.

Map the level index (0–7) linearly onto the same yellow scale. The background can also
step from nothing to `Rgb(40, 33, 0)` to match the spectrum's active-dot appearance at
higher levels.

### 2. Tap text — flash on each tap

The tap count line (`  tap:N`) currently renders in `dim` unconditionally. It should
flash to `beat_style` (yellow fg / dark-yellow bg) briefly after each tap, the same way
the BPM number flashes on each beat.

`deck.tap.last_tap_wall` already records the wall-clock instant of the most recent tap.
Compute `tap_flash_on` inside `info_line_for_deck`:

```rust
let tap_flash_on = deck.tap.last_tap_wall
    .map_or(false, |t| t.elapsed().as_millis() < 150);
```

Apply `beat_style` to the full tap string when `tap_flash_on` is true, `dim` otherwise.

## Log

Implemented as designed. Tap flash uses a dedicated style constant rather than `beat_style` so it fires on actual taps independent of the BPM beat. Background highlight covers ` tap:N ` (one space each side).
