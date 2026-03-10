# Design: BPM Tap Detection
**Status: Draft**

## Approach

### State
Two new variables in `tui_loop`:
- `tap_times: Vec<f64>` — track position in seconds at each tap (using `smooth_display_samp / sample_rate`)
- `last_tap_wall: Option<Instant>` — wall clock of last tap, for 2-second session reset

### On each `b` press
1. If `last_tap_wall` is set and elapsed > 2s: clear `tap_times`.
2. Push current `smooth_display_samp / sample_rate as f64` onto `tap_times`.
3. Update `last_tap_wall = Some(Instant::now())`.
4. If `tap_times.len() >= 8`: call `compute_tap_bpm_offset(&tap_times)` and apply results to `base_bpm` and `offset_ms`. Do not touch `bpm`.

### BPM and offset computation
`compute_tap_bpm_offset(tap_times: &[f64]) -> (f32, i64)`:

**BPM**: derive inter-tap intervals (`windows(2)`), take the median interval as the beat period, convert to BPM.

**Offset**: for each tap time `t`, compute phase `t % beat_period`. Find the circular mean of these phases (convert to unit-circle vectors, average, convert back). This gives the most likely beat anchor in seconds, yielding `offset_ms`.

Using circular mean (rather than arithmetic mean) handles the wraparound at period boundaries correctly — e.g. taps at 0.02s and 0.98s of a 1s period average to 0.0s, not 0.5s.

### Config change
Update `resources/config.toml`: `open_browser = "space+a"`. Update help popup accordingly.

### Info bar
While a tap session is active (i.e. `tap_times` is non-empty and last tap was within 2 seconds), append `tap: N` to the info bar where N is `tap_times.len()`. The field disappears when the session resets.

## Tasks

1. ✓ **Impl**: Update `resources/config.toml` `open_browser` to `"space+a"`; update help popup text.
2. ✓ **Impl**: Add `compute_tap_bpm_offset()` helper; add `tap_times`/`last_tap_wall` state; handle `b` key — reset, push, compute+apply.
3. ✓ **Impl**: Add `tap: N` to info bar when session is active.
4. ✓ **Verify**: tapping updates `base_bpm` and `offset_ms` after 8 taps; count shows and clears; 2s gap resets; `bpm` unaffected; works while playing and paused.
5. ✓ **Process**: Archive
