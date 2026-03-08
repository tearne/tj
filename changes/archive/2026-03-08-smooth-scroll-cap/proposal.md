# Proposal: Smooth Scroll Cap
**Status: Abandoned**

## Intent
Occasional OS scheduler delays cause the smooth display position to advance by an oversized amount in a single frame, producing a visible jump in the scrolling waveform (observed 2–4 times per second). Capping the elapsed time used to advance the smooth position prevents these jumps; the gentle drift correction already in place recovers any resulting lag within a few frames.

## Outcome
Abandoned. Experimentation showed no visible difference. The root cause of the stutter was identified as irregular frame timing relative to column boundaries, not frame-time jitter or audio burst correction. Addressed by the adaptive-framerate proposal instead.

## Specification Deltas

### MODIFIED
- **Smooth display position**: The per-frame wall-clock advance is capped at 2× the current frame period before being applied. Frames delayed by the OS scheduler are treated as if they took at most that duration; any resulting lag behind the real audio position is recovered by the existing gentle drift correction.
