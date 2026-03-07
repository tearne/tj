# Design: Beat Jump
**Status: Ready for Review**

## Approach

### Seekable TrackingSource
Replace `std::vec::IntoIter<f32>` with `Arc<Vec<f32>>` + `Arc<AtomicUsize>` (position counter). The audio thread calls `next()` which does `fetch_add(1, Relaxed)` on the counter; seeking is a single `store(target, SeqCst)`. The audio thread never pauses — the next `next()` call after a seek reads from the new position.

### SeekHandle
A `SeekHandle` struct holds clones of the same `Arc`s and is passed to `tui_loop`:

```rust
struct SeekHandle {
    samples: Arc<Vec<f32>>,
    position: Arc<AtomicUsize>,
    sample_rate: u32,
    channels: u16,
}
```

Methods:
- `current_pos() -> Duration` — derives position from the counter: `pos / (sample_rate * channels)`
- `seek_to(&self, target_secs: f64)` — converts to frame-aligned sample index, snaps to nearest zero crossing within ±10ms, clamps to `[0, samples.len()]`, stores atomically

### Zero-crossing snap
Search `±(sample_rate * channels / 100)` samples (~10ms) around the target. At each frame boundary (index divisible by `channels`), compute the sum of absolute values across all channels. Pick the frame with the minimum amplitude. This minimises the discontinuity without requiring sign-change detection, which is more robust for interleaved stereo.

### Position tracking
Replace every `player.get_pos()` call in `tui_loop` with `seek_handle.current_pos()`. This stays accurate after seeks since it's derived from the actual sample counter, not rodio's internal timer.

### Beat unit state
Add `beat_unit_idx: usize` (default 2 = 16 beats) and `const BEAT_UNITS: &[u32] = &[4, 8, 16, 32, 64, 128]`. Keys `1`–`6` set the index. `[`/`]` compute `jump_secs = BEAT_UNITS[idx] as f64 * 60.0 / bpm as f64` and call `seek_handle.seek_to(current ± jump_secs)`.

### UI
Update the BPM/offset line to include the beat unit: `BPM: 128   offset: +0ms   unit: 16`.

### main() wiring
After decode, wrap `stereo` in `Arc<Vec<f32>>`. Construct `TrackingSource` and `SeekHandle` from shared `Arc`s. Pass `SeekHandle` to `tui_loop`.

## Tasks
1. ✓ Impl: refactor `TrackingSource` to `Arc<Vec<f32>>` + `Arc<AtomicUsize>`; add `SeekHandle` with `current_pos` and `seek_to`
2. ✓ Impl: replace `player.get_pos()` with `seek_handle.current_pos()` in `tui_loop`; update `main()` wiring to build `Arc` and `SeekHandle`
3. ✓ Impl: add `BEAT_UNITS`, `beat_unit_idx` state, `1`–`6` handlers, `[`/`]` handlers, update BPM line and key hints
4. ✓ Verify: jump forward/backward at all units; confirm seamless audio; confirm clamping at track boundaries; confirm position display stays accurate after seek
5. ✓ Process: confirm ready to archive
