# Design: Calibration UX Improvements
**Status: Approved**

## Approach

### Latency adjustment keys (d/c)
Add `Action::LatencyIncrease` / `Action::LatencyDecrease` bound to `d` / `c` in config. In the handler, only apply when `calibration_mode` is true (otherwise the key falls through to nudge). Remove the `calibration_mode` branch from the `OffsetIncrease`/`OffsetDecrease` handlers so `+`/`_` no longer touch latency.

### Latency range 0–500ms
Replace wrapping arithmetic with `.clamp(0, 500)`. Snap on load/entry still rounds to nearest 10ms. Update the latency indicator column formula: `latency_col = (audio_latency_ms as f64 / 500.0 * (dw - 1) as f64).round() as usize` — left edge = 0ms, right edge = 500ms.

### Playhead at normal position during calibration
Remove the `if calibration_mode { dw / 2 } else { ... }` branch for `centre_col`. Always use the configured `playhead_position`.

### Info bar during calibration
Replace the entire left+right group construction with a single line when `calibration_mode`:
```
lat:30ms  d/c adjust  ~ exit
```
Build as a single left_spans, empty right_spans, so the spacer logic still works cleanly.

### Help overlay update
Remove `+`/`_` latency reference. Add `d`/`c` latency line in help text.

## Tasks

1. ✓ **Impl**: Add `LatencyIncrease`/`LatencyDecrease` to Action enum, ACTION_NAMES, config, and handlers. Remove calibration branch from `OffsetIncrease`/`OffsetDecrease`.
2. ✓ **Impl**: Clamp latency to 0–500ms; update latency indicator column formula.
3. ✓ **Impl**: Remove calibration-mode playhead centering.
4. ✓ **Impl**: Replace info bar with calibration-only content when `calibration_mode`.
5. ✓ **Impl**: Update help text.
6. **Verify**: Latency adjusts with d/c only in calibration. +/_ no longer affect latency. Indicator spans full width. Playhead stays at normal position. Info bar clean.
7. **Process**: Confirm ready to archive.
