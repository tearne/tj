# Proposal: Calibration Display Improvements
**Status: Approved**

## Intent

Two small improvements to make the calibrated latency setting more visible and useful during normal playback:

1. **Info bar**: show the current `audio_latency_ms` value persistently in the info bar (outside calibration mode), so the user can confirm calibration is active without having to enter calibration mode.

2. **Latency indicator line**: the vertical line in calibration mode that marks where the next click will arrive at the playhead is currently `│` in `DarkGray` — very subtle against the blank waveform area. Make it more legible.

## Specification Deltas

### MODIFIED

- The info bar right group shall include a `lat:Xms` field showing the current `audio_latency_ms` value. Shown only when `audio_latency_ms > 0`; omitted when 0. Calibration mode is unchanged (it already shows `lat:Nms` as part of its dedicated info bar).

- The latency indicator line in calibration mode shall use `⣿` (U+28FF, full braille block) in `Color::Rgb(80, 100, 140)` (dim steel blue), replacing the current `│` in `DarkGray`. *(Originally proposed as `┆` U+2506; changed to `⣿` after review for better visibility.)*
- Zoom in/out (`-`/`=`) is disabled while calibration mode is active. On entering calibration mode the zoom resets to the default level (4s); on exit the previous zoom level is restored.
