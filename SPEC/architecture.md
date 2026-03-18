# Architecture

## Threading

- Audio decode runs on a background thread with progress reported to the TUI via atomics; the TUI render loop starts immediately and remains responsive during decode.
- Hash computation and BPM detection run on a further background thread after decode, communicating results to the TUI via a channel.
- Audio playback runs on a dedicated thread.
- TUI rendering runs on a separate thread.
- State is shared between threads via lock-free or minimal-contention primitives to meet real-time rendering requirements.

## Caching

- Detected BPM, phase offset, and cue points are cached in `~/.local/share/tj/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format.
- The cache also stores: the last browser directory; `audio_latency_ms` as a single global value.
- Each cache entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- On quit, the current phase offset is persisted to the cache. `audio_latency_ms` is saved on each change and on quit.
