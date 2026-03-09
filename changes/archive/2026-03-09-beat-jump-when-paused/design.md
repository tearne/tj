# Design: Beat Jump When Paused
**Status: Draft**

## Approach

`seek_to` works by setting `fade_remaining = -FADE_SAMPLES` and letting the audio thread execute the jump after the fade-out completes. While paused, rodio stops calling `next()`, so the fade never runs and the seek is never applied.

Fix: add a `seek_direct` method on `SeekHandle` that writes the target position directly into the `position` atomic, also resetting `fade_remaining` and `pending_target` to their idle state. At the `[` / `]` key handlers, call `seek_direct` when paused and `seek_to` when playing (which preserves the click-free fade for live playback).

`seek_direct` reuses the same quiet-frame search logic as `seek_to` so the behaviour is consistent.

## Tasks
1. ✓ Impl: Add `SeekHandle::seek_direct` — direct position write, bypassing the fade
2. ✓ Impl: Update `[` / `]` handlers to call `seek_direct` when paused, `seek_to` when playing
3. ✓ Verify: beat jump works while paused; fade-seek still used during playback
4. ✓ Process: confirm ready to archive
