# Design: Calibration Display Improvements
**Status: Approved**

## Approach

Two isolated one-line changes:

1. **Info bar**: in the right-group span builder (inside `!calibration_mode`), append `  lat:Xms` after the nudge mode field when `audio_latency_ms > 0`.

2. **Latency line**: in the calibration rendering loop, change the character from `'\u{2502}'` (`│`) to `'\u{28FF}'` (`⣿`) and the colour from `Color::DarkGray` to `Color::Rgb(80, 100, 140)`. *(Originally proposed as `┆` U+2506; changed to full braille block `⣿` U+28FF after review for better visibility.)*

3. **Zoom disable + reset**: guard `Action::ZoomOut` and `Action::ZoomIn` handlers with `!calibration_mode`. On calibration entry, save `zoom_idx` to `pre_calibration_zoom_idx` and reset to `DEFAULT_ZOOM_IDX`; on exit, restore `pre_calibration_zoom_idx`.

## Tasks

1. ✓ Impl: add `lat:Xms` to normal-mode info bar right group
2. ✓ Impl: update latency indicator line character and colour
3. ✓ Impl: disable zoom and reset to default on calibration entry; restore on exit
4. ✓ Process: archive
