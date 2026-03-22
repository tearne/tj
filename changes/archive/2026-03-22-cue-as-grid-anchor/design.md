# Design: Cue as Beat Grid Anchor
**Status: Approved**

## Approach

Five discrete changes, all in `src/main.rs` and `resources/config.toml`.

### 1. Cue set snaps the beat grid

In the `Deck1Cue`/`Deck2Cue` space-chord handlers (currently around lines 755–770), after setting `d.cue_sample`, call `anchor_beat_grid_to_cue(d)` before the cache save. The cache save must also persist the updated `offset_ms` — the current code only spreads `cue_sample` into the entry, so it needs `offset_ms: d.tempo.offset_ms` added to the `CacheEntry` update.

### 2. Key swap in config

In `resources/config.toml`, swap the bindings:
- `deck1_cue = "A"` (was `"space+a"`)
- `deck1_cue_play = "space+a"` (was `"A"`)
- Same for deck 2.

### 3. Rework `Deck1Cue` / `Deck2Cue` as regular actions

`Deck1Cue` is currently handled only in the `space_held` branch (hardcoded check at line 751). After the key swap it must work as a regular key action instead.

- Change the space-chord branch to check for `Deck1CuePlay`/`Deck2CuePlay` instead of `Deck1Cue`/`Deck2Cue`, with the new maintain-play-state seek logic (see §4).
- Move the cue-set logic into the regular action handler, replacing the current no-op at line 1427. New behaviour: only act when paused — set `cue_sample`, call `anchor_beat_grid_to_cue`, persist to cache. Do nothing when playing.

### 4. `Deck1CuePlay` / `Deck2CuePlay` — maintain play state

Update the seek logic (currently in the regular action handler around line 1428, and now also in the space-chord branch) to:
- If no cue is set: do nothing.
- Seek to cue position.
- If was playing: resume play (set `smooth_display_samp` to `cue + latency`; call `player.play()`).
- If was paused: stay paused (set `smooth_display_samp` to `cue`; no play call).

The regular action handler entry for `Deck1CuePlay`/`Deck2CuePlay` is kept so the action works if a user binds it to a non-space-chord key.

### 5. Remove BPM tap confirmation

In the `Deck1BpmTap` / `Deck2BpmTap` handlers, remove the `cue_tap_pending` block entirely — tapping now proceeds unconditionally regardless of cue state. Remove the `cue_tap_pending` expiry in `service_deck_frame`. Remove the `cue_tap_pending` field from the `Deck` struct and its initialisation in `Deck::new`.

## Tasks

1. ✓ Impl: Cue set — call `anchor_beat_grid_to_cue` after setting cue; persist `offset_ms` in cache save
2. ✓ Impl: Config — swap `deck1_cue` / `deck1_cue_play` and deck2 equivalents in `config.toml`
3. ✓ Impl: Rework `Deck1Cue`/`Deck2Cue` — move from space-chord branch into regular action handler (paused-only set); update space-chord branch to dispatch `Deck1CuePlay`/`Deck2CuePlay`
4. ✓ Impl: `Deck1CuePlay`/`Deck2CuePlay` — update seek logic to maintain play state
5. ✓ Impl: Remove BPM tap confirmation — handlers, `service_deck_frame` expiry, `Deck` field
6. ✓ Process: Confirm ready to archive
