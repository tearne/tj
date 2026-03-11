# Design: BPM Manual Entry
**Status: Approved**

## Approach

Add two new `Action` variants (`BaseBpmIncrease`, `BaseBpmDecrease`) mapped to `F` and `V` by default. Their handlers mirror the `BpmHalve`/`BpmDouble` pattern: adjust `base_bpm`, set `bpm = base_bpm`, call `player.set_speed(1.0)`, and persist to cache.

## Tasks

1. ✓ **Bug**: Reset `space_held` when a chord fires so terminals without key-release events don't get stuck.
2. **Impl**: Add `BaseBpmIncrease` / `BaseBpmDecrease` to the `Action` enum.
3. **Impl**: Add default key bindings (`F` → `base_bpm_increase`, `V` → `base_bpm_decrease`) to `ACTION_NAMES` and the default config.
4. **Impl**: Add handlers — clamp `base_bpm` ±0.1, set `bpm = base_bpm`, `player.set_speed(1.0)`, persist to cache.
5. **Verify**: `F`/`V` adjusts `base_bpm` and display; resets any f/v offset; persists to cache.
6. **Process**: Archive
