# Cue End-of-Track Reset

## Intent
When a track finishes playing and there is a cue point set, the playhead should return to the cue point rather than to the start of the track. This matches the expected behaviour: load cue → play → end → return to cue ready for another play.

## Approach
In `service_deck_frame` in `src/main.rs`, the end-of-track block resets the seek position and display sample to `0.0`. Change both to use the cue sample position when one is set, falling back to `0.0` when no cue is set.

The `seek_direct` call takes seconds; compute `cue_secs` from `d.cue_sample` and `d.audio.sample_rate`. The `smooth_display_samp` uses mono samples directly.

## Plan
- [x] UPDATE IMPL — `service_deck_frame` end-of-track block: reset to cue position when `d.cue_sample` is `Some`

## Conclusion
Single change to the end-of-track block in `service_deck_frame` (`src/main.rs`). When `d.cue_sample` is set, both `seek_direct` and `smooth_display_samp` now reset to the cue position; otherwise they fall back to `0.0` as before.
