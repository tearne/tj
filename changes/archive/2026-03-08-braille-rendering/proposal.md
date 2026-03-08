# Proposal: Direct Braille Rendering
**Status: Ready for Review**

## Intent
Eliminate the per-frame Canvas rasterization cost that causes waveform stutter. Ratatui's Canvas widget recomputes the entire Braille dot grid from scratch every frame regardless of whether the waveform data changed. Replacing it with pre-rendered Braille characters (computed once per data change, in background threads) removes this work from the render loop entirely.

## Specification Deltas

### MODIFIED

**Waveform Visualisation:**
- The detail and overview waveforms are rendered as pre-computed Braille characters rather than via ratatui's Canvas widget. The visual result is identical.
- The Braille character grids are recomputed in background threads whenever the source data or canvas dimensions change (including on window resize). The UI render loop performs no dot rasterization — it only assigns per-column colours and passes the result to a `Paragraph` widget.
- Beat markers and the playhead/centre-line are rendered by colouring the relevant columns of the pre-computed grid (DarkGray for beat/bar columns, White for the playhead/centre), rather than as separate Canvas draw calls.
- On window resize, both waveform views update automatically because the background threads detect the dimension change and re-render.
