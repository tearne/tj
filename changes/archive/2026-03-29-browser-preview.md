# Browser Preview

## Intent

While browsing, the user has no way to audition a track before loading it onto a deck. This change adds a preview key (`#`) that begins playback of the highlighted track immediately from a fixed position ~20% through the file. Any subsequent keypress stops the preview. The feature is designed for low latency: the goal is that audio starts within a fraction of a second of the keypress.

## Approach

**Latency strategy** — the dominant source of latency is opening the audio device (50–200ms on ALSA/PulseAudio). A dedicated preview output is opened when the browser opens and kept alive (silent) until the browser closes, so the per-preview cost is only file open + seek + initial buffer fill (~10–30ms total). Decode is streaming (symphonia decoder wrapped as a rodio `Source`), not pre-decoded, so playback starts without waiting for the full file to be read.

**Seek position** — fixed at 20% of the track duration. Duration is read from the symphonia format probe (available from file headers for FLAC and most MP3s without scanning the body). If duration is not available from the probe, the preview starts from a fixed 30-second offset; if the file is shorter than 30s it starts from the beginning.

**Key behaviour**
- `#` on a highlighted audio file: start preview (or restart if one is already running)
- `#` on a directory entry: no-op
- any other key while preview is running: stop preview first, then handle the key normally
- browser close (Esc, Enter, q): stop any active preview

**Output** — a separate rodio sink opened at browser entry, distinct from the two deck players. Normal stereo playback; no channel routing or filtering. Torn down on browser close.

**State** — `tui_loop` gains `preview_output: Option<PreviewOutput>` alongside the existing deck state. `PreviewOutput` holds the sink and a handle to the currently playing source (if any). Created on browser open, dropped on browser close.

**Scope** — no effect on deck state, BPM analysis, or the cache.

## Plan

- [x] ADD `SymphoniaPreviewSource` in `audio/mod.rs`: streaming rodio `Source` backed by a symphonia decoder; constructed from a path + byte offset (seek position derived from duration probe)
- [x] ADD `PreviewOutput` struct in `audio/mod.rs`: wraps a `rodio::Player`; `play(path)` stops any current source and appends a new `SymphoniaPreviewSource`; `stop()` clears the player
- [x] UPDATE `tui_loop`: create `PreviewOutput` on browser open; drop on browser close; pass reference into browser key handler
- [x] UPDATE browser key block in `tui_loop`: `#` → `preview.play(path)`; any other key → `preview.stop()` then handle key; close paths stop preview before returning
- [x] UPDATE `SPEC/browser.md`: `#` key entry and preview behaviour description
- [x] UPDATE version bump (patch)
- [x] UPDATE hint bar in `render_browser`: `#: preview` added to both directory and search-results hints

## Conclusion

Added instant browser track preview. Pressing `#` on a highlighted audio file begins streaming playback from 20% into the track (30s offset fallback) via a dedicated `SymphoniaPreviewSource` and `PreviewOutput` connected to the main mixer. The audio device is kept warm for the duration of the browser session, so per-keypress latency is file-open + seek only (~10–30ms). Any other key stops the preview before its normal action runs. The `#` key is shown in the bottom hint bar in both directory and search-results modes.
