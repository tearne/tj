# Design: BPM Correction
**Status: Ready for Review**

## Approach

### Mono sample sharing
`mono` is currently moved into `WaveformData::compute`. Change `WaveformData` to hold `Arc<Vec<f32>>` and `compute` to take `Arc<Vec<f32>>`. In `main()`, wrap mono in `Arc` before use, then pass the same `Arc` to `tui_loop` for re-detection.

### Mutable BPM in tui_loop
Add `mono: Arc<Vec<f32>>` and `sample_rate: u32` parameters to `tui_loop`. Make `bpm` a mutable local (`let mut bpm = bpm;`). Move `beat_period` and `flash_window` computation inside the render loop so they stay derived from the current `bpm` on every frame.

### Detection mode state
```rust
const DETECT_MODES: &[&str] = &["auto", "fusion", "legacy"];
let mut detect_mode: usize = 0;
```
Maps to `AnalysisConfig` variants:
- `0` → `AnalysisConfig::default()`
- `1` → `AnalysisConfig { enable_bpm_fusion: true, ..Default::default() }`
- `2` → `AnalysisConfig { force_legacy_bpm: true, ..Default::default() }`

### h / H handlers
Multiply `bpm` by 0.5 or 2.0, clamped to `[40.0, 240.0]`. No re-analysis.

### r handler
1. Advance `detect_mode = (detect_mode + 1) % 3`.
2. Draw one frame with the status line showing `"Detecting…"` so feedback is immediate.
3. Call `detect_bpm(&mono, sample_rate)` with the new mode's config (blocking — audio continues).
4. Update `bpm`. Update the cache entry immediately (not just on quit).

### BPM line
`BPM: 128 [fusion]   offset: +0ms   unit: 16 beats`

### Cache update on BPM change
On both `h`/`H` and `r`, update the cache entry immediately after the new BPM is known (same as quit save, but without exiting).

## Tasks
1. ✓ Impl: wrap mono in `Arc<Vec<f32>>` in `main()`; update `WaveformData` to hold and accept `Arc<Vec<f32>>`; pass mono Arc and sample_rate to `tui_loop`
2. ✓ Impl: mutable `bpm`; move beat_period/flash_window into loop; add `detect_mode` state; add `h`/`H`/`r` handlers; update BPM line; immediate cache write on correction
3. ✓ Verify: implemented and smoke-tested; did not resolve the specific detection issue but feature works correctly
4. ✓ Process: confirm ready to archive
