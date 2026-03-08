# Proposal: Background Loading & Analysis
**Status: Ready for Review**

## Intent
Eliminate the two main expected UI freezes: the loading screen during track decode, and the BPM analysis block on `r`. In both cases the TUI should remain responsive, and audio should start as soon as decode completes regardless of whether BPM analysis is finished.

## Specification Deltas

### MODIFIED

**Track loading:**
- The TUI render loop starts immediately on launch and remains responsive throughout loading (window resize and other terminal events are handled).
- Decode runs on a background thread. A loading screen displays a progress bar showing decode progress (samples decoded vs estimated total from codec params).
- As soon as decode completes, playback begins and the player view is shown — the user does not wait for BPM analysis to start hearing audio.
- Hash computation and BPM detection (or cache lookup) continue on a background thread after decode.
- While BPM analysis is in progress, the player view is fully functional — waveform, seek, zoom, transport all work normally. Beat markers are suppressed and beat jump uses a 120 BPM placeholder. The BPM line shows an animated indicator (e.g. `BPM: --- [analysing]`).
- When analysis completes, the BPM updates, beat markers appear, and beat jump uses the detected tempo. If a cache entry is found, this transition happens quickly (hash lookup only); if not, it takes the full analysis duration.

**Re-analysis (`r` key):**
- `r` no longer blocks. It spawns analysis on a background thread and returns immediately.
- While re-analysis is in progress, beat markers are suppressed and the BPM line shows the animated indicator. The player continues normally.
- When the result arrives, the BPM updates and markers reappear. Cache is written immediately.

### ADDED
- A background analysis state is represented in the UI: BPM line shows `BPM: --- [analysing ▸]` (or similar) while detection is pending, and the current value once known.
