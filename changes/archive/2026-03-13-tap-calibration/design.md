# Design: Live Latency Adjustment
**Status: Approved**

## Approach

Add two new actions `LatencyDecrease` / `LatencyIncrease` bound to `[` / `]`. Each performs a compound adjustment: `audio_latency_ms` changes by ±10ms and `offset_ms` is compensated by ∓10ms (then wrapped into `[0, beat_period_ms)`). The waveform display shifts while tick markers stay anchored to their heard position. Both values persist to cache immediately. Only active outside calibration mode.

The existing calibration mode (`~`) is unchanged. `[`/`]` are ignored inside calibration mode to avoid conflicting with `d`/`c`.

## Tasks

1. ✓ Impl: add `LatencyDecrease` / `LatencyIncrease` actions, key bindings, and handlers
2. ✓ Impl: update help overlay to include `[`/`]`
3. ✓ Process: archive
