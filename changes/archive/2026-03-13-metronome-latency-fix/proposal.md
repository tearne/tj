# Proposal: Metronome Latency Fix
**Status: Approved**

## Intent

The metronome click was firing at the wrong time relative to `audio_latency_ms`. Clicks were injected when the *speaker* position reached the beat, meaning they arrived at the speaker one full latency period late. Increasing `audio_latency_ms` made the problem worse.

## Specification Deltas

### MODIFIED
- The metronome fires based on the audio buffer write position (ahead of the speaker by `audio_latency_ms`), not the display position. This ensures the click arrives at the speaker exactly on the beat when latency is correctly calibrated.
