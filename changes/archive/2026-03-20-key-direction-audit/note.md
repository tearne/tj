# Note: Tick Offset Key Direction

User reports that the tick offset keys feel the wrong way around. BPM key inversion
(s/x, f/v) is intentional — matches turntable/CDJ pitch fader convention.

## Current mapping

| Key | Action | Physical position |
|---|---|---|
| `!` (Shift+1) | `offset_ms += 10` — grid shifts 10ms **later** | Row 1 (top) |
| `Q` (Shift+q) | `offset_ms -= 10` — grid shifts 10ms **earlier** | Row 2 (below) |

Same pattern for Deck 2: `£` (Shift+3) = increase, `E` (Shift+e) = decrease.

## The question

Upper key (`!`) shifts the beat grid **later** in the track. Lower key (`Q`) shifts it
**earlier**.

Is "later" the natural direction for the upper key? Arguments either way:

- **Consistent**: every other non-BPM vertical pair uses upper = forward/increase
  (filter, level, jumps, nudge). Offset follows the same rule.
- **Counterintuitive**: in practice, when a tick mark appears slightly *before* the
  beat, the corrective action is to press `!` (upper) to shift the grid later — but
  it might feel more natural to press the *lower* key to "pull the tick back" toward
  the beat, i.e. associating the key position with the desired movement of the marker
  rather than the arithmetic direction of offset_ms.

## Proposed experiment: stationary ticks, moving waveform

When the user adjusts the offset, the current visual is: ticks jump, waveform stays
put. The proposed alternative: ticks stay visually fixed, waveform shifts instead.

The relative motion is identical — but the metaphor changes from "I am repositioning
the grid" to "I am sliding the audio under the grid." If the latter feels natural, the
key directions are correct. If it still feels wrong, the keys should be swapped.

**Implementation sketch**: when offset_ms changes by Δms, simultaneously shift
`smooth_display_samp` by the equivalent sample count in the opposite direction. Net
effect: tick positions on screen are unchanged, the waveform content shifts. The offset
change still takes effect in the audio engine as normal.

This only affects the display update at the moment of the keypress — no persistent
state change beyond what already happens.
