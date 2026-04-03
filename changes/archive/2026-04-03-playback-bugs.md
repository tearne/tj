# Playback Bugs

## Intent

Four playback bugs to address:

1. **Cue Jump not working** — `Space+R` does not seek to the cue point as expected.

2. **Pitch shift inverted** — `A` lowers pitch when it should raise it; `Z` raises when it should lower. The fix is to swap the `pitch_up` and `pitch_down` bindings so `A` maps to `pitch_up` and `Z` to `pitch_down`.

3. **Pitch reset label wrong in keymap** — `Space+Z` / `Space+A` correctly reset pitch to zero, but the keyboard help overlay labels the Space layer of Z and A as `+Ptch` (suggesting pitch up) rather than `Ptch=`.

4. **Pitch shift moves the beat grid** — Applying pitch shift displaces the beat tick positions on the waveform display rather than leaving them in place and only changing the audio pitch. The likely cause is that `PitchSource` batch-reads 512 frames at a time from `TrackingSource`, causing the position counter to jump ahead by 512 samples the moment pitch is applied, shifting the displayed playback position relative to the beat grid.

## Approach

### 1. Cue Jump

The early intercept at `src/main.rs:1187` correctly seeks to the cue sample, then sets `space_held = false`. Execution falls through to the normal action dispatch; with `space_held` now false, `R` is looked up as a plain key (jump −8 bars) which immediately overrides the cue seek. Fix: add `continue` after the seek in the early intercept to skip the rest of that key event's processing.

### 2. Pitch shift inverted

`pitch_up = "z"` and `pitch_down = "a"` in `resources/config.toml` are swapped relative to what the user expects. Fix: swap them to `pitch_up = "a"` and `pitch_down = "z"`. Update `SPEC/config.md` accordingly.

### 3. Pitch reset label

The Space-layer cells for Z and A in the `key-rebinding.md` planning sketch read `+Ptch` / `-Ptch` rather than `Ptch=`. This is a planning document error only — the `?` modal correctly shows `Space+Z or Space+A reset`, and the binding itself works. Fix: correct the sketch in `key-rebinding.md` and ensure `render_keyboard_help` uses `Ptch=` when implemented.

### 4. Pitch shift moves beat grid

`PitchSource` batch-reads 512 frames at a time from `TrackingSource` to feed SoundTouch. Each batch causes `TrackingSource.position` to jump ahead by 512 samples (~11 ms at 44 100 Hz), shifting the displayed waveform position — and with it the beat tick phase — the moment pitch is applied. Fix: introduce a separate output-position counter that `PitchSource` increments for each sample it emits; the display and `SeekHandle` should use this counter rather than `TrackingSource`'s consumed-sample counter.

Review cadence: per task.

## Plan

- [x] FIX `src/main.rs`: add `continue` in the CuePlay early intercept after the seek so the normal action dispatch is skipped
- [x] FIX `resources/config.toml`: swap `pitch_up` and `pitch_down` bindings (`a` ↔ `z`)
- [x] UPDATE `SPEC/config.md`: swap the pitch +/− labels for A and Z
- [x] UPDATE `changes/active/planning/key-rebinding.md`: correct Space-layer labels for Z and A from `+Ptch`/`-Ptch` to `Ptch=`
- [x] FIX `src/audio/mod.rs`: add an output-position counter to `PitchSource` and update `SeekHandle` (and any display-position reads) to use it instead of `TrackingSource`'s position

## Log

- `key-rebinding.md` no longer exists in `changes/active/planning/` — task was a no-op.
- SPEC correction used `=Ptch` per user preference (not `Ptch=` as written in the plan).
- CuePlay while playing: after initial build, display did not snap to new position because `seek_to` did not update `output_position`. Fixed by also storing `target_sample` into `output_position` in `seek_to`, so the drift correction fires immediately.
- End-of-track detection and remaining-time read kept on `TrackingSource.position` (not `output_position`) — `output_position` accumulates across seeks and would falsely trigger end-of-track.

## Conclusion

All four bugs fixed. `output_position` is used only for display drift correction; `TrackingSource.position` is retained for end-of-track detection and remaining time. The pitch fix requires a release build and removal of any existing `~/.config/deck/config.toml` if auto-created by an older binary.
