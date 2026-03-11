# Design: Tap-Guided BPM Re-detection
**Status: Approved**

## Approach

### Segment extraction
At the end of each tap update (when `tap_times.len() >= 8`), extract a mono sample slice:
- `pad_secs = 60.0 / tapped_bpm` (one beat of padding each side)
- `start = ((tap_times[0] - pad_secs) * sample_rate).max(0.0) as usize`
- `end   = ((tap_times.last() + pad_secs) * sample_rate).min(mono.len()) as usize`
- `segment = mono[start..end].to_vec()`

### AnalysisConfig
```rust
AnalysisConfig {
    enable_bpm_fusion: true,
    min_bpm: tapped_bpm * 0.95,
    max_bpm: tapped_bpm * 1.05,
    legacy_bpm_preferred_min: tapped_bpm * 0.95,
    legacy_bpm_preferred_max: tapped_bpm * 1.05,
    ..AnalysisConfig::default()
}
```

### Background thread
Same pattern as `BpmRedetect`: spawn a thread, create a new `(tx, rx)` channel, replace `bpm_rx`. The thread runs `analyze_audio(&segment, sample_rate, config)` and sends `(hash, result.bpm, offset_snap)` — where `hash` is the full-track hash captured at launch time (so the result saves to the correct cache entry), and `offset_snap` is a placeholder (the receiver will override it).

### Preserving tap offset
Add `tap_offset_pending: Option<i64>` to `tui_loop` state:
- Set to `Some(offset_ms)` when tap-guided detection is launched.
- Cleared to `None` when the tap session resets (`tap_times.clear()`).

In the `bpm_rx` receiver, check `tap_offset_pending`:
- `Some(tap_offset)` → use `tap_offset` for `offset_ms` instead of the value from the analyser; clear `tap_offset_pending`.
- `None` → a tap reset occurred while detection was in flight; discard the result entirely (do not update `base_bpm` or `offset_ms`).

### Animated indicator
Set `analysis_hash = None` when launching, same as `BpmRedetect`. The spinner appears and beat markers are suppressed until the result arrives.

## Tasks

1. ✓ **Impl**: Add `tap_offset_pending: Option<i64>` state; on `BpmTap` with 8+ taps, extract segment, build config, spawn background thread, set `analysis_hash = None` and `tap_offset_pending = Some(offset_ms)`.
2. ✓ **Impl**: Clear `tap_offset_pending` when tap session resets.
3. ✓ **Impl**: Update `bpm_rx` receiver to check `tap_offset_pending` — preserve tap offset if `Some`, discard result if `None`.
4. **Verify**: After tapping, spinner shows; `base_bpm` updates to analyser precision; `offset_ms` remains from tap; resetting tap session before result arrives discards it.
5. **Process**: Archive
