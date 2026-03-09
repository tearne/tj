# Design: Detail View Tick Visibility
**Status: Draft**

## Approach

Increase tick mark colour from `DarkGray` to `Gray` in the detail view tick row renderer. The half-column characters (⡇/⢸) are retained — using `0xFF` (⣿) was investigated and rejected because isolated full-width bytes lose half-column precision, causing the tick to oscillate ±0.5 col relative to the waveform on every sub-column step.

## Tasks
1. ✓ Impl: Change tick row colour from `Color::DarkGray` to `Color::Gray`
2. ✓ Verify: tick marks more visible at all zoom levels; sync with waveform preserved
3. ✓ Process: confirm ready to archive
