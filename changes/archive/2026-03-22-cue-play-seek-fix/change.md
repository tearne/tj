# Cue Play Seek Fix
**Type**: Fix
**Status**: Approved

## Log

Cue play while playing used `seek_direct` (paused-only) and called `player.play()` on an already-playing deck, causing sync loss. Fix: use `seek_to` when playing (same as beat jump) and remove the spurious `play()` call. When paused, `seek_direct` is correct and unchanged.

When playing, the seek target is shifted forward by `audio_latency_ms` worth of samples (`cue_samp + latency_samps`). Since the speaker trails the write head by `latency_samps`, this lands the speaker exactly on `cue_samp` after the seek.
