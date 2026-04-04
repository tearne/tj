# Beat Jump Display Glide

## Intent

When playing and pressing a single beat jump key (1 or Q), the waveform display slowly glides to the new position over ~0.8 seconds rather than snapping immediately. The audio seeks correctly; only the display is wrong.

## Approach

The display position (`smooth_display_samp`) is tracked separately from the audio position (`output_position`). After a seek, `output_position` is updated immediately. A drift correction in `service_deck_frame` compares the two and either snaps (large drift) or slowly corrects (small drift at 5% per frame).

The snap threshold is `sample_rate * 0.5` (500ms). A 1-beat jump at 130 BPM moves ~462ms — just under the threshold — so the slow correction applies instead, producing the glide.

Fix: lower the threshold to `sample_rate * 0.1` (100ms). This is comfortably above normal timing jitter (< 20ms accumulated drift during playback) and comfortably below the smallest conceivable beat jump (1 beat at 400 BPM ≈ 150ms).

One line change in `src/main.rs` at the `large_drift` assignment.

Review cadence: per task.

## Plan

- [x] FIX `src/main.rs`: change `* 0.5` to `* 0.1` in the `large_drift` threshold

## Conclusion

One-line fix applied. Snap threshold lowered from 500ms to 100ms, ensuring a 1-beat jump at any reasonable BPM (up to ~600 BPM) triggers an immediate snap rather than a slow display glide.
