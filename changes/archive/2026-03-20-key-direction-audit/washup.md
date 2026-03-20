# Washup: Key Direction Audit

## Intent

Investigated whether the beat offset keys (`!`/`Q`, `£`/`E`) felt the wrong way around.
Ran an experiment changing the visual metaphor: ticks stay fixed, waveform shifts on
offset adjust. User confirmed the metaphor feels natural, validating that key directions
are correct.

Also confirmed BPM key inversion (`s`/`x`, `f`/`v`) is intentional — matches
turntable/CDJ pitch fader convention.

## Spec delta

**Offset adjust while paused** (new behaviour):

When the track is paused and the user presses an offset key, `smooth_display_samp` and
the audio position both shift by the raw ±10ms step. The net effect: tick screen positions
are unchanged; the waveform content shifts by ±10ms instead. This reinforces the metaphor
"slide the audio under the beat grid."

When playing, offset changes have no effect on the display position (drift correction
would undo any shift before the next frame).

**Key directions confirmed correct** — no binding changes needed:

| Key | Action |
|---|---|
| `!` / `£` | `offset_ms += 10` — waveform shifts 10ms earlier (grid moves later relative to audio) |
| `Q` / `E` | `offset_ms -= 10` — waveform shifts 10ms later (grid moves earlier relative to audio) |
