# Design: Metronome First Click
**Status: Approved**

## Approach

When `MetronomeToggle` enables the metronome, `last_metro_beat` is `None`. The per-frame check fires immediately on the next frame because `None != Some(beat_index)`. The fix: pre-set `last_metro_beat = Some(beat_index)` at the moment of activation, so the current beat is already marked seen and the first click fires on the *next* beat transition.

`beat_index` is computed earlier in the same loop iteration (before the draw closure) and is in scope at the action dispatch site, so no structural changes are needed.

## Tasks

1. ✓ Impl: update `MetronomeToggle` arm to initialise `last_metro_beat = Some(beat_index)` on enable
2. ✓ Process: archive
