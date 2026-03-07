# Proposal: BPM Cache
**Status: Ready for Review**

## Intent
Replace per-file sidecar files (`.tj`) with a single user-level cache file, keyed by a stable content hash of the audio. This makes BPM and offset data portable across renames and moves, eliminates clutter in music directories, and centralises cache management.

## Specification Deltas

### ADDED

**BPM Cache**:
- A cache file is maintained at `~/.local/share/tj/cache.json`.
- Each entry is keyed by a Blake3 hash of the decoded mono PCM samples, making it invariant of filename, tags, and container format.
- Each entry stores:
  - `bpm` — detected BPM (f32)
  - `offset_ms` — user-adjusted phase offset (i64)
  - `name` — the filename at the time of first detection, stored as a human-readable hint to aid manual cache management (informational only; not used as a key)
- The cache is read on load (to skip re-detection) and written after BPM detection and on quit (to persist offset adjustments).

### REMOVED

- Sidecar files (`.tj` alongside the audio file) are no longer created or read.
