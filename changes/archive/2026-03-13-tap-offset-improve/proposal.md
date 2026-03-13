# Proposal: Improve Tap-Informed Offset
**Status: Note**

## Intent

The current tap-derived offset is not performing as well as expected. The exact nature of the problem is not yet fully characterised — this note captures the investigation starting point.

## Current Behaviour

`compute_tap_bpm_offset` derives offset via a mean residual approach: each tap's deviation from the nearest beat boundary (anchored to the first tap) is averaged and added back to the first tap's position. BPM uses the median inter-tap interval.

## Potential Issues to Investigate

- **First-tap anchor sensitivity**: the offset is anchored to `tap_times[0]`. If the first tap is an outlier (early/late), the entire phase shifts. A more robust anchor (e.g. the tap closest to the mean residual, or the median tap) may reduce sensitivity.
- **Mean vs median residual**: averaging residuals is sensitive to outliers. A median residual could be more robust for sloppy taps.
- **10ms snap after derivation**: the snap to the nearest 10ms introduces up to 5ms of error. Whether this is perceptible at typical BPMs is worth considering.
- **Interaction with re-detection**: the tap offset is preserved when re-detection completes, but the re-detected BPM may not align perfectly with the tap phase — the offset is correct for the tapped BPM but the re-detected BPM is slightly different.
- **User timing bias**: humans tend to tap slightly late. A fixed correction (e.g. −20ms) has been used in some metronome apps. Worth evaluating whether a bias adjustment improves perceived alignment.

## Observed Problems

- More taps don't improve accuracy — the offset keeps shifting rather than converging.
- After re-detection completes, the beat markers reappear but land in unexpected positions. This may be partly because the track has moved on and accumulated drift has worsened since the tap session ended.
- The instability makes it hard to evaluate whether tapping is helping at all.

## Resolution

Three changes implemented:

1. **BPM via linear regression**: replaced median inter-tap interval with least-squares regression of tap index vs tap time. Converges as taps accumulate; later taps add leverage and reduce variance.

2. **Outlier tap filter**: after a first-pass regression, taps with residual > half a beat period are dropped before a second-pass regression. Handles occasional mis-taps without disrupting the session.

3. **Re-detection removed**: background re-analysis after tap session was confirmed as the primary instability source (taps-only was noticeably more stable). Re-detection path and associated dead code (`tap_guided_rx`, `tap_offset_pending`, `compute_tap_offset_for_bpm`) removed.
