# Experiment: Stationary Ticks on Offset Adjust
**Status: Adopted**

## Question

When the user adjusts the beat offset, should the ticks stay visually fixed and the
waveform shift, rather than the ticks jumping and the waveform staying put? Would this
make the key directions feel more natural?

## Implementation

When paused: on offset change, call `set_position` to shift the audio position by the
same delta. `smooth_display_samp` snaps to the new audio position in the same frame
(via the paused drift-snap logic). Since both offset and display position moved by the
same delta, tick screen positions are unchanged — the waveform content shifts instead.

When playing: no change to existing behaviour (a display shift would be undone by drift
correction before the next render anyway).

## Log

- v0.5.107: Initial implementation. Shifted `smooth_display_samp` by `new_offset - old_offset`
  and called `set_position`. The metaphor felt right to the user, confirming key direction
  is correct. Two bugs found: tick wobble (half-character) and waveform full rerender on
  offset wrap.
- v0.5.108: Fixed both bugs. Tick wobble: `detail_view_start` now uses exact
  `display_pos_samp` instead of quantized buffer anchor. Rerender: delta hardcoded to
  raw ±10ms rather than `new_offset - old_offset` (which was wrong across rem_euclid wraps).

## Outcome

Adopted. The stationary-ticks metaphor — "slide the audio under the grid" — feels natural
and confirms the key directions are correct. The behaviour is now permanent: pressing an
offset key while paused shifts the waveform by ±10ms; ticks remain visually fixed.
