# Cache Entry Construction

## Intent

`CacheEntry` is constructed inline at many call sites in `main.rs`. Every new persisted field requires updating each site individually — demonstrated by the `offset_established` change, which touched eight construction sites across the file. This is fragile and noisy.

The sites also have inconsistent shape: some use struct spread (`..entry`) to carry forward fields from the existing cache entry; others construct in full. The spread pattern was introduced to avoid forgetting fields, but it only partially addresses the problem.

## Approach

Introduce `fn cache_entry_for_deck(d: &Deck) -> CacheEntry` in `src/deck/mod.rs`. This function is the single authoritative translation from deck state to a `CacheEntry`. All `cache.set(hash, CacheEntry { ... })` call sites in `main.rs` become:

```rust
cache.set(hash, cache_entry_for_deck(d));
```

Adding a new persisted field then requires only: adding it to `CacheEntry`, adding it to `Deck`, and updating `cache_entry_for_deck` — one place.

`CacheEntry.name` (human-readable filename hint) is currently passed as `d.filename.clone()` at every call site; `cache_entry_for_deck` includes it automatically.

The `..entry` spread pattern was used to preserve fields not yet reflected on the deck. After this refactor all fields are on the deck so the spread is no longer needed. The one exception is the analysis result site, which restores `cue_sample` and `offset_established` from cache immediately before writing — this pattern stays, but the write becomes `cache_entry_for_deck(d)`.

Scope: `src/deck/mod.rs` (add function), `src/main.rs` (replace all inline constructions). No behaviour change.

## Plan

- [x] ADD IMPL `cache_entry_for_deck(d: &Deck) -> CacheEntry` in `src/deck/mod.rs`
- [x] UPDATE IMPL replace all inline `CacheEntry { ... }` constructions in `src/main.rs` with `cache_entry_for_deck(d)`
- [x] REVIEW compile and verify no behaviour change

## Conclusion

Added `cache_entry_for_deck` to `src/deck/mod.rs` as the single authoritative translation from deck state to `CacheEntry`. Replaced all 12 inline constructions in `src/main.rs`. The spread pattern sites (`..entry`) and their associated `cache.get` guards were eliminated entirely — every call site is now a single `cache.set(hash.clone(), cache_entry_for_deck(d))`. No behaviour change; build clean.
