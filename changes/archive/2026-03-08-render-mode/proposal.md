# Proposal: Switchable Detail Waveform Render Mode
**Status: Ready for Review**

## Intent

The detail waveform currently uses a pre-rendered 3× buffer with a sliding viewport. This keeps the UI thread lightweight but means the waveform grid is recomputed infrequently (roughly once per screen-width of playback). An alternative mode re-renders the braille grid fresh every frame, centred exactly on the current smooth display position.

The two modes offer different trade-offs. With the smooth display position fix in place, both should scroll without jumping; the per-frame mode may feel more "alive" or offer a subtly sharper waveform near the edges. The user should be able to toggle between modes at runtime to compare and choose a preference.

## Specification Deltas

### ADDED

- The detail waveform supports two render modes, toggled at runtime with a key (e.g. `m`):
  - **Buffer mode** (default): a buffer wider than the visible area is pre-rendered on the background thread; the UI thread slides a viewport through it each frame. Recomputes only on zoom, resize, or large seek.
  - **Live mode**: the braille grid is recomputed every frame, centred on the current smooth display position. Each frame reflects the exact playback position with no residual offset from a stale buffer.
- The current mode is shown in the UI (e.g. in the key hints or status line).

### MODIFIED

- **Rendering**: the stable-buffer principle (see Rendering section) applies to buffer mode. In live mode, recomputation occurs every frame by design; smoothness relies entirely on the smooth display position.
