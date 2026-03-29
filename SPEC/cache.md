# Cache

## Overview

Detected BPM, phase offset, and cue points are cached in `~/.config/deck/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format.

## Contents

- Per-track entries: BPM, phase offset (`offset_ms`), cue point sample position. Keyed by Blake3 hash of decoded mono samples.
- Each entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- Last visited browser directory.
- Browser workspace directory (the root used for fuzzy search). Absent if none has been set; silently discarded on load if the directory no longer exists.
- `audio_latency_ms` as a single global value shared across both decks.
- `vinyl_mode` as a single global boolean. Defaults to `false` (beat mode) for a fresh installation.

## Persistence

- Loaded at startup before the browser or player opens.
- Per-track BPM and offset corrections are persisted immediately on each change.
- `audio_latency_ms` is saved on each change and on quit.
- `vinyl_mode` is saved immediately on each toggle.
- On quit, the current phase offset for each loaded deck is persisted.
- The cache file is never modified automatically for keys not explicitly updated by the application.
