# Browser Preview

## Intent

While browsing, the user has no way to audition a track before loading it onto a deck. This change lets the user hold `#` on a highlighted entry to preview it in the left (monitor) channel from a random position between 20% and 80% through the track, stopping as soon as the key is released.

## Approach

**Hold detection** — `#` `Press` starts the preview; `#` `Release` stops it. Because terminal Release events are not always delivered, the preview is also stopped by any other keypress while the browser is open, and by browser close.

**Decoding** — preview decode runs on a background thread (same pattern as `start_load`). While decoding is in progress, `#` is a no-op. On completion the decoded stereo samples are seeked to the random position and played. Random position uses `SystemTime::now().subsec_nanos()` as a seed — no new dependency.

**Left-channel routing** — a `PreviewSource` wrapper implements `rodio::Source` over the decoded stereo interleaved samples, zeroing the right channel on every frame. It is appended to the existing mixer so it shares the same output device.

**State** — `tui_loop` holds `preview_state: Option<PreviewHandle>` where `PreviewHandle` wraps a `rodio::Player` and a stop flag. On `#` Release (or any other key / close), `player.stop()` is called and the handle dropped.

**Scope** — preview only triggers when the highlighted entry is an audio file, not a directory. Preview does not affect deck state, BPM analysis, or the cache.

## Plan

- [ ] ADD `PreviewSource` in `audio/mod.rs`: stereo `Source` over `Arc<Vec<f32>>` with right channel zeroed and a seek offset
- [ ] ADD `start_preview(path) -> PendingPreview` background decode function (mirrors `start_load`)
- [ ] ADD `PreviewHandle` struct: `Player` + `Arc<AtomicBool>` stop flag
- [ ] UPDATE `tui_loop`: `pending_preview` and `preview_handle` state; poll completion each frame; connect to mixer
- [ ] UPDATE browser key block in `tui_loop`: `#` Press → `start_preview`; `#` Release → stop; any other key → stop
- [ ] UPDATE browser close paths (Esc, Enter, q) to stop any active preview
- [ ] UPDATE SPEC/browser.md: `#` key entry, preview behaviour
- [ ] UPDATE version bump
