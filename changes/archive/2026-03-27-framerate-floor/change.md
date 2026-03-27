# Framerate Floor
**Type**: Fix
**Status**: Approved

## Log
- Raise minimum framerate from 5 fps (200ms cap) to 20 fps (50ms cap)
- When tag editor is open, override frame_dur to 16ms (~60 fps) so text navigation and input are never throttled by waveform zoom level
- Halve spectrum analyser update interval (quarter-beat instead of half-beat; 250ms fallback instead of 500ms when analysing)
