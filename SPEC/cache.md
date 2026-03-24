# Cache

## Overview

Detected BPM, phase offset, and cue points are cached in `~/.local/share/tj/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format.

## Contents

- Per-track entries: BPM, phase offset (`offset_ms`), cue point sample position. Keyed by Blake3 hash of decoded mono samples.
- Each entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- Last visited browser directory.
- `audio_latency_ms` as a single global value shared across both decks.

## Persistence

- Loaded at startup before the browser or player opens.
- Per-track BPM and offset corrections are persisted immediately on each change.
- `audio_latency_ms` is saved on each change and on quit.
- On quit, the current phase offset for each loaded deck is persisted.
- The cache file is never modified automatically for keys not explicitly updated by the application.
