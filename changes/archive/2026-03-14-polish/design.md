# Design: Polish
**Status: Approved**
*(retrospective)*

## Approach
Addressed all compiler warnings by removing dead code and suppressing intentional-but-unused items.

## Changes Made
- Removed `MICRO_FADE_SAMPLES` constant (unused; comment claimed use for micro-jumps not yet implemented)
- Removed `file_dir` from `tui_loop` signature and its computation at the call site (passed but never used)
- Removed `space_chord_fired` variable and all three assignment sites (tracked but never read)
- Removed `time_str` dead computation and its `fmt_dur` closure (both only existed to produce each other)
- Removed `overview_width` dead local in mouse click handler
- Removed `total_mono_samps` dead local in end-of-track check
- Removed `mut` from `left_spans` (not mutated after initialisation)
- Added `#[allow(dead_code)]` to `NotificationStyle` enum (`Warning` and `Error` variants are intentional future infrastructure)
