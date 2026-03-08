# Proposal: Smooth Buffer Handoff in Buffer Mode
**Status: Ready for Review**

## Intent

When the background thread recomputes the braille buffer, the waveform visibly jumps on the next frame. The jump occurs because each buffer divides its audio window into columns using `chunk_size = window.len() / buf_cols`, which varies slightly between recomputes as the window start shifts. Even a 1-sample change in chunk_size moves every column boundary, producing different peak values for the same audio positions — a visible discontinuity on swap.

## Specification Deltas

### MODIFIED

- **Rendering**: Buffer recomputes in buffer mode are visually seamless. The waveform continues scrolling through the buffer handoff without any jump or flicker.
