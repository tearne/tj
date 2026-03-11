# Design: Audio Latency Calibration
**Status: Approved**

## Approach

### State
- `audio_latency_ms: i64` — global, loaded from cache on startup, default 0.
- `calibration_mode: bool` — toggled by `~`.

### Display position offset
All rendering that currently uses `smooth_display_samp` shifts by the latency:
```rust
let display_samp = smooth_display_samp - audio_latency_ms as f64 * sample_rate as f64 / 1000.0;
```
`display_samp` replaces `smooth_display_samp` in: waveform viewport, beat tick positions, beat flash phase, overview playhead.

### Calibration pulse
A fixed 120 BPM metronome independent of the track. Pulse period = `sample_rate * 60 / 120` samples = 0.5s.

**Click tone**: synthesise a short tone (e.g. 1kHz sine, ~20ms, with a fast attack/decay envelope) and inject into the mixer on each pulse, same pattern as `scrub_audio`.

**Travelling marker**: each pulse fires at a fixed wall-clock time. The marker's position in the detail waveform is computed the same way as beat tick marks, but using the pulse grid rather than `base_bpm`. Rendered as a distinct bright colour (e.g. `Color::Cyan`) so it stands out from normal beat ticks.

**Playhead flash**: when the nearest pulse is within half a column of the playhead centre, flash the playhead to a bright colour (e.g. `Color::White` → `Color::Yellow`).

### `+`/`-` in calibration mode
In the `OffsetIncrease`/`OffsetDecrease` handlers, check `calibration_mode`: if true, adjust `audio_latency_ms` instead of `offset_ms`.

### Cache
Add `audio_latency_ms: i64` as a top-level field in `cache.json` (alongside `last_browser_path` and track entries). Load on startup, save on change and on quit.

### Info bar
Append `  lat:Nms` to the info bar when calibration mode is active.

## Tasks

1. ✓ **Impl**: Add `audio_latency_ms` to cache (load/save).
2. ✓ **Impl**: Compute `display_samp` from `smooth_display_samp - latency` and thread it through all rendering.
3. ✓ **Impl**: Add `calibration_mode` state and `~` key binding (`CalibrationToggle` action).
4. ✓ **Impl**: Synthesise click tone and schedule injection at 120 BPM pulse times while calibration is active.
5. ✓ **Impl**: Render travelling pulse markers in the detail waveform and flash the playhead on coincidence.
6. ✓ **Impl**: Redirect `+`/`-` to adjust `audio_latency_ms` in calibration mode; show `lat:Nms` in info bar.
7. **Verify**: Latency offset visibly shifts waveform/markers; pulses travel through detail view; playhead flashes on hit; value persists across sessions.
8. **Process**: Archive
