# Waveform Vertical Symmetry
**Type**: Fix
**Status**: Approved

## Problem

The waveform renders bottom-heavy: at certain amplitudes the bottom lobe is one
braille-dot taller than the top lobe for a symmetric signal.

The cause is in `render_braille`. Both `top_dot` and `bot_dot` are computed with
floor (integer truncation). For a symmetric signal (`min = −max`), `top_f + bot_f
= total_dots` exactly at round amplitude multiples. Flooring both pulls them both
toward the top — `top_dot` gains one dot upward, `bot_dot` also shifts upward (i.e.
away from the bottom), so the bottom ends up one dot short of matching the top:

| Amplitude | top_dot | bot_dot | Top half | Bot half |
|-----------|---------|---------|----------|----------|
| 0.50      | 5       | 15      | 5 dots   | 6 dots ← |
| 0.45      | 5       | 14      | 5 dots   | 5 dots ✓ |
| 0.30      | 7       | 13      | 3 dots   | 4 dots ← |

(20-dot panel: 5 braille rows × 4 dots, centre between dots 9 and 10.)

## Fix

When `bot_f` lands on an exact integer, floor includes that boundary dot but
the equivalent calculation on the top excludes it. Subtract 1 from `bot_dot`
when `bot_dot + top_dot >= total_dots`:

```rust
let bot_dot = {
    let raw = (((1.0 - clamped_min) / 2.0 * total_dots as f32) as usize)
        .min(total_dots - 1);
    if raw > top_dot && raw + top_dot >= total_dots { raw - 1 } else { raw }
};
```

The `raw > top_dot` guard preserves the single-dot render for zero-amplitude
columns (`top_dot == raw == total_dots / 2`).

`render_braille` is shared by detail and overview waveforms; both are fixed.

## Log

`render_braille` fix was already applied from a prior session. Testing revealed a
second asymmetry: the background renderer stored the full panel height (`h`) in
`shared_renderer.rows`, but deck A's display clips to `h - 1` waveform rows (the
bottom row is the shared tick row). This meant deck A's buffer was rendered one row
taller than what it could display, clipping the bottom of the waveform.

Fix: store `h - 1` (waveform height) in `shared_renderer.rows`. Both buffers now
render at the same height. Deck A displays all `h - 1` waveform rows plus the tick
row. Deck B iterates `h` rows but the buffer has `h - 1`; the last row returns
`None` and renders as a blank braille row (background colour).
