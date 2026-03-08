# Design: Waveform Detail Performance
**Status: Ready for Review**

## Approach

A dedicated background thread owns the `detail_peaks` computation. It polls the current position and zoom at ~120 Hz (8ms sleep), recomputing only when the displayed window would actually change — i.e. when the centre column index or zoom level differs from the last computed value. The result is stored as `Arc<Vec<(f32, f32)>>` behind a `Mutex`; the UI thread clones the `Arc` (a single atomic increment) at the start of each frame and passes it into the canvas closure by move — no audio sample iteration on the UI thread at all.

### Shared state (created in `tui_loop`, arcs passed to background thread)

| Variable | Type | Writer | Reader |
|---|---|---|---|
| `detail_cols` | `Arc<AtomicUsize>` | UI thread (each frame) | background thread |
| `detail_zoom` | `Arc<AtomicUsize>` | UI thread (each frame) | background thread |
| `detail_peaks` | `Arc<Mutex<Arc<Vec<(f32,f32)>>>>` | background thread | UI thread |
| `stop_detail` | `Arc<AtomicBool>` | UI thread (on exit) | background thread |

### Background thread loop

```
loop:
  if stop → break
  cols = detail_cols.load(); zoom = detail_zoom.load()
  pos_sample = position.load() / channels          // reuse SeekHandle's position Arc
  col_samples = (zoom_secs * sample_rate / cols).max(1)
  center_col  = pos_sample / col_samples

  if cols | zoom | center_col changed since last compute:
      new_peaks = waveform.detail_peaks(pos_dur, zoom_secs, cols)   // allocation here, off UI thread
      *detail_peaks.lock() = Arc::new(new_peaks)

  sleep 8ms
```

The lock is held only for the pointer swap (Arc clone assignment), not during computation.

### UI thread changes

- At the top of the render loop, before `terminal.draw`:
  - Store `zoom_idx` → `detail_zoom` atomic
- Inside `terminal.draw`, after layout (where `dw` is known):
  - Store `dw` → `detail_cols` atomic
  - Clone the shared `Arc<Vec<(f32,f32)>>` from the Mutex (one lock acquisition, one refcount bump)
  - Pass the cloned Arc into the canvas `.paint()` closure by move
- Remove the direct `waveform.detail_peaks()` call from the render path

### Initialisation / teardown

- Before entering `tui_loop`'s main loop, spawn the background thread.
- `stop_detail` is set to `true` in every early-return path from `tui_loop`.
- An initial empty `Arc<Vec<(f32,f32)>>` is stored so the first render (before the thread's first compute) shows a flat line rather than panicking.

### SeekHandle position access

The background thread needs the current sample position. `SeekHandle` already holds `Arc<AtomicUsize>` for `position` and carries `sample_rate` and `channels`. Rather than duplicating these, pass `Arc::clone(&seek_handle.position)`, `sample_rate`, and `channels` directly to the background thread.

## Tasks

1. ✓ Impl: Add shared atomics and `Arc<Mutex<Arc<Vec<(f32,f32)>>>>` to `tui_loop`; spawn background detail-peak thread that recomputes on position/zoom/cols change and sleeps 8ms between polls.
2. ✓ Impl: Update the render loop to write `detail_cols`/`detail_zoom` atomics each frame and read the cached `Arc<Vec<(f32,f32)>>` instead of calling `waveform.detail_peaks()`.
3. Verify: `cargo build`; manual test — waveform renders correctly, stutter eliminated or significantly reduced.
4. Process: Confirm ready to archive.
