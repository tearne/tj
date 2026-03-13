# Proposal: Live Latency Adjustment
**Status: Approved**

## Intent

Provide a way to tune `audio_latency_ms` during normal playback by watching the tick markers shift relative to waveform peaks, as an alternative to the existing click-based calibration mode which requires paused playback. The user taps to lock in BPM, then nudges latency until ticks visually align with the beats in the waveform.

## Specification Deltas

### ADDED

- `[` and `]` perform a compound latency adjustment during normal playback (outside calibration mode): `audio_latency_ms` changes by ±10ms while `offset_ms` is simultaneously compensated by ∓10ms. This keeps tick markers anchored to their heard position (as set by tapping) while the waveform display shifts, making the tick/waveform misalignment visible. When ticks align with waveform peaks, latency is correctly calibrated.
- `audio_latency_ms` is clamped to 0–250ms; `offset_ms` is wrapped into `[0, beat_period_ms)` after each adjustment.
- Both values are persisted to the cache immediately.
- `lat:Xms` in the info bar right group provides continuous feedback.

## Notes

- The existing click-based calibration mode (`~`) is unchanged and complementary — it is more precise but requires paused playback.
- No new mode is introduced; `[`/`]` work at any time outside calibration mode.
