# Design: BPM Cache
**Status: Ready for Review**

## Approach

### Dependency
Add `blake3 = "1"` to `Cargo.toml`. Blake3 is fast, has no unsafe requirements for our use, and produces a 32-byte digest we encode as a 64-char hex string for use as the cache key.

### Cache structure
Replace the `Sidecar` struct with a `Cache` struct:

```rust
// ~/.local/share/tj/cache.json
// { "<hex-hash>": { "bpm": 128.0, "offset_ms": -20, "name": "track.flac" }, ... }

struct CacheEntry {
    bpm: f32,
    offset_ms: i64,
    name: String,
}

struct Cache {
    path: PathBuf,
    entries: HashMap<String, CacheEntry>,
}
```

`Cache::load(path)` reads and deserialises the file, returning an empty map on any error.
`Cache::save()` serialises and writes atomically (write to a temp file, rename).
`Cache::get(hash)` looks up an entry.
`Cache::set(hash, entry)` inserts/updates an entry.

### Hash computation
A new `fn hash_mono(samples: &[f32]) -> String` casts the `f32` slice to bytes via `bytemuck::cast_slice` (already an indirect dependency; use `unsafe` raw bytes cast if not available) and feeds it to `blake3::hash()`, returning the hex string.

### main() flow changes
- After decode, compute hash from mono samples.
- Load cache; look up hash.
  - Hit: use stored `bpm` and `offset_ms`.
  - Miss: run BPM detection; insert entry with current filename; save cache.
- On quit: update the entry's `offset_ms`; save cache.
- Remove all sidecar (`Sidecar`) code.

### Cache file location
`dirs::home_dir()` would add a dependency. Instead, use `std::env::var("HOME")` with a fallback to `std::env::current_dir()` — no extra crate needed. Directory is created on first write if it doesn't exist.

## Tasks
1. ✓ Impl: add `blake3` to `Cargo.toml`; add `hash_mono` + `Cache`/`CacheEntry` structs with load/save/get/set
2. ✓ Impl: wire into `main()` — compute hash, cache lookup, detection, quit-save; remove `Sidecar`
3. ✓ Verify: build clean; smoke-test: first play writes cache entry; second play loads from cache; offset persists; rename file and confirm cache still hits
4. ✓ Process: confirm ready to archive
