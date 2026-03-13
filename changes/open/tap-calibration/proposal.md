# Proposal: Tap-Based Latency Calibration
**Status: Note**

## Intent

The improved tap BPM (linear regression) converges reliably with enough taps. This suggests an alternative calibration workflow: tap in time with a known track until BPM stabilises, then adjust `audio_latency_ms` until the tick markers visually align with the waveform peaks. This would complement (or replace) the current synthetic-click calibration, which requires the user to listen for coincidence of a click and a playhead flash.

## Notes

- Requires a reliable convergence signal — some indicator of when the BPM estimate has stabilised enough to trust (e.g. variance of recent regression residuals falling below a threshold).
- The user would adjust latency while observing the waveform rather than listening for a click, which may be more intuitive.
- Interacts with the existing calibration mode (`~`). Could be a new mode or an extension of the existing one.
- Depends on tap improvements being stable first.
