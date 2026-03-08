# Proposal: Waveform Detail Performance
**Status: Ready for Review**

## Intent
Eliminate the periodic stutter in the detail waveform view. The current render loop calls `detail_peaks()` on every frame, which allocates a fresh `Vec` and iterates over O(zoom × sample_rate) audio samples. Occasional allocator jitter from the per-frame allocation is the most likely cause of the 2–3 stutter events per second observed in both debug and release builds.

## Specification Deltas

### MODIFIED

**Waveform Visualisation:**
- The detail view peak envelope is computed on a dedicated background thread and rendered from a cached result, so the UI render loop performs no audio sample iteration.
- The cache is invalidated and recomputed whenever the zoom level changes or the playback position has advanced by at least one column's worth of samples (i.e. when the rendered output would actually differ).
