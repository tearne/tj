# Cache Entry Construction
**Type**: Proposal
**Status**: Approved

## Problem

`CacheEntry` is constructed inline at many call sites in `main.rs`. Every new persisted field requires updating each site individually — demonstrated by the `offset_established` change, which touched eight construction sites across the file. This is fragile and noisy.

The sites also have inconsistent shape: some use struct spread (`..entry`) to carry forward fields from the existing cache entry; others construct in full. The spread pattern was introduced to avoid forgetting fields, but it only partially addresses the problem.

## Proposed Fix

Introduce `fn cache_entry_for_deck(d: &Deck) -> CacheEntry` in `src/deck/mod.rs`. This function is the single authoritative translation from deck state to a `CacheEntry`. All `cache.set(hash, CacheEntry { ... })` call sites in `main.rs` become:

```rust
cache.set(hash, cache_entry_for_deck(d));
```

Adding a new persisted field then requires:
1. Adding it to `CacheEntry` in `src/cache/mod.rs`
2. Adding it to `TempoState` (or `Deck`) in `src/deck/mod.rs`
3. Updating `cache_entry_for_deck` — one place

**Secondary cleanup**: `CacheEntry.name` is the human-readable filename hint, currently provided as `d.filename.clone()` at every construction site. Since `d.filename` is already on the deck, `cache_entry_for_deck` includes it automatically — no caller needs to supply it.

**Spread sites**: the `..entry` pattern was used to preserve fields not yet reflected on the deck. After this refactor, all fields are on the deck, so the spread is no longer needed. The one exception is the analysis result site, which restores `cue_sample` and `offset_established` from cache immediately before writing — this pattern stays, but the write becomes `cache_entry_for_deck(d)`.

## Scope

- `src/deck/mod.rs`: add `cache_entry_for_deck`
- `src/cache/mod.rs`: no structural changes
- `src/main.rs`: replace all inline `CacheEntry { ... }` constructions with `cache_entry_for_deck(d)`
- No behaviour change
