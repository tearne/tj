# Proposal: Calibration UX Improvements
**Status: Ready for Review**

## Intent
Make calibration mode more usable: dedicated adjustment keys, inline help, and a cleaner info bar.

## Specification Deltas

### MODIFIED
- Latency adjustment in calibration mode: `d` / `c` (increase / decrease), replacing the overloaded `+` / `_` keys.
- `audio_latency_ms` range: 0–500ms (was ±500ms). Negative latency is meaningless and removed. Clamped at 0 on the low end; wrapping arithmetic removed.
- The latency indicator bar spans 0–500ms (left = 0ms, right = 500ms), doubling the visual resolution per ms compared to the previous ±500ms range.
- The playhead remains at its normal configured position during calibration (no longer forced to centre). The calibration pulse marker still travels toward the playhead.
- Info bar during calibration shows only: `lat:Nms  d/c adjust  ~ exit`.
- All other info bar fields are hidden during calibration.

## Detail

### Latency adjustment keys
`d` / `c` adjust `audio_latency_ms` while in calibration mode (increase / decrease). These keys are free during calibration since nudge has no meaningful effect there. The existing `+` / `_` bindings continue to adjust beat phase offset in normal mode and are no longer overloaded for latency.

### Inline help in info bar during calibration
Replace or augment the current `lat:Nms  ~ to exit` text with a brief inline hint showing the active keys, e.g.:
```
lat:30ms  d/c adjust  ~ exit
```
This removes the need to open the `?` overlay during calibration.

### Remove irrelevant info during calibration
The info bar shows only calibration-relevant content. Hidden during calibration: play/pause icon, BPM, beat phase offset, nudge mode, zoom, level, filter indicator, spectrum strip. Shown: `lat:Nms  d/c adjust  ~ exit`.
