# Proposal: Info Bar Fixes
**Status: Draft**

## 1 — Latency always visible

### Problem
`audio_latency_ms` is rendered in the info bar as `lat:Xms` only when `audio_latency_ms > 0`. At the default value of 0 the field is absent, giving the user no indication that a latency setting exists or what its current value is.

### Change
Always render the latency field — `lat:0ms` at zero, `lat:Xms` otherwise. Remove the `if audio_latency_ms > 0` guard in `info_line_for_deck`.

---

## 2 — Deck 2 controls not syncing struct fields when deck 1 is empty

### Problem
The empty-deck handler has two cases where it updates audio state but not the corresponding `Deck` struct field that rendering reads:

**Level** (`Deck2Level*`, lines ~812–815): calls `player.set_volume()` but doesn't update `d.volume`. The info bar level glyph reads `deck.volume`, so it stays frozen.

**Filter** (`Deck2Filter*`, lines ~816–818): updates `filter_offset_shared` but not `d.filter_offset`. The spectrum is computed from `d.filter_offset`, so the spectrum display doesn't respond to filter keys.

The main handler correctly updates both the struct field and the audio/shared state in both cases.

### Change
In the empty-deck handler, sync struct fields alongside the audio state updates:

**Level** — update `d.volume` as source of truth, then call `player.set_volume(d.volume)`:
```
Deck2LevelUp   → d.volume = (d.volume + 0.05).min(1.0); d.audio.player.set_volume(d.volume);
Deck2LevelDown → d.volume = (d.volume - 0.05).max(0.0); d.audio.player.set_volume(d.volume);
Deck2LevelMax  → d.volume = 1.0; d.audio.player.set_volume(d.volume);
Deck2LevelMin  → d.volume = 0.0; d.audio.player.set_volume(d.volume);
```

**Filter** — update `d.filter_offset` alongside `filter_offset_shared`:
```
Deck2FilterIncrease → d.filter_offset = (d.filter_offset + 1).min(16); d.audio.filter_offset_shared.store(d.filter_offset, ...);
Deck2FilterDecrease → d.filter_offset = (d.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.filter_offset, ...);
Deck2FilterReset    → d.filter_offset = 0; d.audio.filter_offset_shared.store(0, ...);
```
