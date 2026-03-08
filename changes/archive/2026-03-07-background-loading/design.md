# Design: Background Loading & Analysis
**Status: Ready for Review**

## Approach

Three separate blocking operations currently freeze the TUI: the decode loop in `main()`, the hash+BPM block in `main()`, and the `analyze_audio` call on `r`. Each is moved to a background thread and replaced with a polling loop on the main/TUI thread.

### Decode with progress bar

`decode_audio` is refactored to accept two `Arc<AtomicUsize>` arguments — `decoded_samples` and `estimated_total_samples` — which it updates as each packet is decoded. `estimated_total_samples` is initialised from `track.codec_params.n_frames * channels` before the loop begins (falls back to 0 if unavailable, giving indeterminate progress).

The decode runs on a `std::thread::spawn` thread that sends the result `(mono, stereo, sample_rate, channels)` through an `mpsc::channel`. The main thread enters a loading render loop:
- Draws a `Paragraph` header + ratatui `Gauge` widget showing `decoded / estimated` ratio (clamped 0–1; shows full bar if estimated is 0).
- Polls events at 30ms cadence (matching `tui_loop`) for window resize (handled automatically by ratatui) and `q` to abort.
- Calls `result_rx.try_recv()` each iteration; breaks on `Ok`.

### Background BPM detection

Immediately after decode completes the main thread:
1. Computes `WaveformData` (fast, stays synchronous).
2. Starts the `Player` with a `TrackingSource` — audio begins playing.
3. Spawns a second thread that computes the Blake3 hash, does the cache lookup or `detect_bpm`, and sends `(f32, i64)` (bpm, offset_ms) through a channel.
4. Calls `tui_loop`, passing the receiver.

`tui_loop` signature gains `bpm_rx: mpsc::Receiver<(f32, i64)>` and loses `initial_bpm` / `initial_offset_ms`. Internally:
- `bpm_state: Option<(f32, i64)>` — `None` while analysing, `Some` once received.
- Each frame, `bpm_rx.try_recv()` is called; on `Ok`, `bpm_state` is set and the cache is written.
- `effective_bpm`: `bpm_state.map(|(b,_)| b).unwrap_or(120.0)` — used for beat calculations and jump.
- `offset_ms`: `bpm_state.map(|(_,o)| o).unwrap_or(0)` — used for phase calculations.
- BPM line: `"BPM: --- [analysing <spinner>]"` when `None`, `"BPM: {bpm:.0} [{mode}]  offset: ..."` when `Some`.
- `draw_bar_ticks` / `draw_beat_lines`: skip drawing (return early) when `bpm_state` is `None`.
- Spinner: cycles a `&[char]` of braille frames (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) indexed by `frame_count % 10`.

### Non-blocking `r` key

`tui_loop` gains `pending_rx: Option<mpsc::Receiver<(f32, i64)>>` replacing `bpm_rx` (they unify). On `r`:
- Advance `detect_mode`.
- Clone `mono` arc and `hash` string into the new thread.
- Spawn a thread that runs `analyze_audio` with the selected config and sends `(bpm, offset_ms)` (preserving current offset_ms).
- Replace `pending_rx` with the new `Some(receiver)`. The old receiver is dropped (its thread result is discarded).
- `bpm_state` is reset to `None` so the `---` indicator reappears immediately.

This unifies initial-analysis and re-analysis into a single polling path: one `Option<mpsc::Receiver<(f32, i64)>>` that is polled every frame.

### Cache write timing

Cache is written when the receiver delivers a result (`bpm_state` transitions from `None` to `Some`). On quit / track switch, if `bpm_state` is `None` (analysis still in progress), no cache write is attempted for this track (no entry to update). If `Some`, current offset is persisted as before.

## Tasks

1. ✓ Impl: Refactor `decode_audio` to take progress atomics; add background-decode loading loop in `main()` with `Gauge` progress bar, event handling, and `q` support.
2. ✓ Impl: Spawn background hash+BPM thread after decode; update `tui_loop` signature to take `mpsc::Receiver<(String, f32, i64)>`; add `analysis_hash: Option<String>`, spinner, suppressed markers, 120 BPM fallback, per-frame `try_recv`.
3. ✓ Impl: Make `r` key non-blocking — spawn analysis thread, reset `bpm_state` to `None`, replace receiver.
4. ✓ Verify: `cargo build`; manual smoke test — loading bar visible; player starts before BPM; `---` shows then updates; `r` returns immediately; beat markers appear after analysis.
5. Process: Confirm ready to archive.
