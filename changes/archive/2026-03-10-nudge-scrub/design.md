# Design: Nudge Scrub
**Status: Draft**

## Approach

### Audio injection
`mixer` (a `rodio::Mixer`) already has an `add(source)` method that injects a `Source` directly into the output mix, independent of the main `player`. `rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples)` wraps a `Vec<f32>` as a one-shot source — exactly what we need for a short scrub snippet.

When paused and a nudge fires:
1. Compute the interleaved sample range:
   - `start = smooth_display_samp as usize * channels`
   - `end = (start + buf.samples_per_col * channels).min(seek_handle.samples.len())`
2. Clone the slice into a `Vec<f32>` and wrap it: `SamplesBuffer::new(channels as u16, sample_rate, slice)`
3. Call `mixer.add(snippet)` — it plays immediately alongside (or after) any other active sources.

No early-termination mechanism is needed. At typical zoom levels a snippet is 10–400ms; even with rapid nudging the overlap is brief and musically informative.

### Passing mixer into tui_loop
`mixer` is currently owned in `main()` and not visible to `tui_loop`. Add it as a parameter: `mixer: &rodio::Mixer`. The `Mixer` type is `Arc`-backed so passing a reference is fine.

### Trigger points
Fire a scrub snippet after updating `smooth_display_samp` in:
- Jump mode: `nudge_backward` and `nudge_forward` handlers, when `player.is_paused()`.
- Warp mode: the per-frame drift block (`else if nudge != 0` while paused), each frame that the nudge key is held.

For warp mode per-frame scrub, only fire a new snippet when the position has advanced by at least one column width since the last scrub, to avoid flooding the mixer at the render frame rate.

## Tasks

1. ✓ **Impl**: Add `mixer: &rodio::Mixer` parameter to `tui_loop`; update call site in `main()`.
2. ✓ **Impl**: Add scrub helper — extract interleaved samples, construct `SamplesBuffer`, call `mixer.add()`.
3. ✓ **Impl**: Call scrub helper in jump-mode nudge handlers (backward and forward) when paused.
4. ✓ **Impl**: Call scrub helper in warp-mode per-frame drift block when paused, throttled to one snippet per column advance.
5. ✓ **Verify**: Paused jump nudge plays a short snippet; paused warp nudge plays continuous scrub without flooding; playing nudge is silent (no scrub); scrub does not interrupt main playback.
6. ✓ **Process**: Archive
