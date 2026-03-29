# Cue on Load

## Intent

When a track finishes loading, the deck is left at position zero even when a cached cue point exists. In beat mode the user must manually seek to the cue before playback, which breaks the expectation that a track loads ready to play.

## Approach

After a `PendingLoad` completes and the deck is built, if the deck is not in vinyl mode and a cached cue point is present, seek the player to that position automatically. No new state is required — the cue is already available on the loaded deck.

The cue is restored from cache in `service_deck_frame` when BPM analysis completes — that is the earliest point where `cue_sample` is known, and where the seek must be placed.

## Plan

- [x] UPDATE IMPL: In `service_deck_frame`, after restoring `cue_sample` from cache, seek to the cue position via `seek_direct` if not in vinyl mode and `cue_sample` is `Some`

## Conclusion

In `service_deck_frame`, after the cached cue is restored on BPM analysis completion, the player now seeks directly to the cue position when not in vinyl mode. Tracks with a cached cue load ready to play.
