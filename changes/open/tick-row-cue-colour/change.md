# Tick Row Cue Colour
**Type**: Fix
**Status**: Draft

## Problem

The shared tick row in `render_detail_waveform` (lines 2287–2292) checks `cue_screen_col`
the same way the waveform rows do, so when the cue marker for deck 1 falls at a column
that is visible in the tick row it renders magenta instead of the tick's normal colour.
The tick row should only ever show the playhead colour (`centre_col`) — cue markers belong
to the waveform rows only.

## Fix

In the tick row loop inside `render_detail_waveform`, remove the two `cue_screen_col`
branches. The colour logic simplifies to:

```
if c == centre_col  →  (white ⣿, playhead)
else if byte != 0   →  (gray, tick braille char)
else                →  (gray, space)
```

No other changes needed — the waveform row loop above it is unaffected.

## Log
