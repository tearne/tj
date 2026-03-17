# Design: Info Bar Fixes
**Status: Complete**

## 1 — Latency always visible

Removed `if audio_latency_ms > 0` guard in `info_line_for_deck`. `lat:0ms` now shown at default.

## 2 — Level and filter sync in empty-deck handler

Fixed four `Deck2Level*` arms and three `Deck2Filter*` arms in the empty-deck handler to update the struct field alongside the audio/shared state, matching the main handler:

- Level: `d.volume` updated first, then `player.set_volume(d.volume)`. Removed stale read from `player.volume()`.
- Filter: `d.filter_offset` updated first, then `filter_offset_shared.store(d.filter_offset, ...)`. Bounds clamped to ±16 (matching main handler; previous code used ±96).
