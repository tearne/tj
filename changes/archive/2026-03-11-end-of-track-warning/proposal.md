# Proposal: End-of-Track Warning
**Status: Ready for Review**

## Intent
When approaching the end of a track, give a visual cue so the user isn't caught off-guard. The bar markers on the overview waveform flash in time with the beat, alternating colour each beat (one beat on, one beat off).

## Specification Deltas

### ADDED
- When the remaining playback time falls below a configurable threshold, the overview bar markers flash in time with the BPM: the marker colour alternates each beat (one beat on, one beat off), using a muted reddish-grey.
- The warning threshold is configurable via `warning_threshold_secs` in the `[display]` section of `config.toml`. Default: `15`.
- The flash is driven by the same beat phase used for the info bar beat-flash, so it stays locked to the beat grid.
- The warning is active only during playback (not while paused).
