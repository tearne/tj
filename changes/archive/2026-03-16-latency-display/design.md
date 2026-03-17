# Design: Latency Display and Offset Compensation
**Status: Complete**

## 1 — Latency moved to global bar

Removed `audio_latency_ms` parameter from `info_line_for_deck` and its render span. Added `lat:Xms` to the idle global bar line (alongside browser path), always shown. Both render paths (empty-deck and main) updated.

## 2 — Offset compensation removed

Removed the `offset_ms` adjustment from `LatencyIncrease` / `LatencyDecrease` handlers. Latency is now a pure display calibration — adjusting it no longer modifies the beat grid or writes to cache.
