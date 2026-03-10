# Design: Waveform Outline Mode
**Status: Draft**

## Approach

`render_braille` currently fills all dots from `top_dot` to `bot_dot` per column. In outline mode, only the dots at `top_dot` and `bot_dot` are set (the two extremes), leaving the interior empty.

### Changes

**`render_braille`**: add a `outline: bool` parameter. When `true`, replace the `for d in top_dot..=bot_dot` fill loop with two point sets — one at `top_dot`, one at `bot_dot` (guarding the case where they're equal so the dot isn't set twice).

**Shared state**: add `detail_style: Arc<AtomicUsize>` (0 = fill, 1 = outline) shared between the UI thread and background rasteriser, alongside the existing `detail_cols`/`detail_rows` atomics.

**Background thread**: read `detail_style` each iteration. Add `style != last_style` to the `must_recompute` condition so a toggle immediately re-renders the buffer. Pass `outline: style == 1` to `render_braille`.

**Overview**: unchanged — `render_braille` is also called for the overview waveform; the `outline` flag is always `false` there.

**Toggle action**: add `WaveformStyle` to the `Action` enum as `WaveformStyle`. On press, flip `detail_style` between 0 and 1. Add `waveform_style` to `ACTION_NAMES` and `resources/config.toml` (unmapped — no entry).

## Tasks

1. ✓ **Impl**: Add `outline: bool` parameter to `render_braille`; update fill loop for outline case; update the two call sites (detail background thread passes flag, overview always passes `false`)
2. ✓ **Impl**: Add `detail_style` atomic; wire toggle action; add to `Action` enum and `ACTION_NAMES`; add `must_recompute` condition in background thread
3. **Verify**: Confirm toggle switches between fill and outline on the detail waveform; confirm overview is unaffected; confirm recompute fires immediately on toggle
4. **Process**: Confirm ready to archive
