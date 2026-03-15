# Design: Load Behaviour and BPM Increment Refinements
**Status: Draft**

## Approach

### Load paused
`Player::connect_new` returns a playing player. A single `player.pause()` call immediately after (line ~217 in `load_deck`) starts the deck paused. No other changes needed — `PlayPause` already toggles correctly.

### BPM increments
Two literal changes:
- `BpmIncrease` / `BpmDecrease`: `± 1.0` → `± 0.01`
- `BaseBpmIncrease` / `BaseBpmDecrease`: `± 0.1` → `± 0.01`

The clamp bounds (40.0–240.0) and all other logic are unchanged.

## Tasks

1. ✓ **Impl**: Pause player immediately after creation in `load_deck`
2. ✓ **Impl**: Change `BpmIncrease`/`BpmDecrease` step from `1.0` to `0.01`; `BaseBpmIncrease`/`BaseBpmDecrease` step from `0.1` to `0.01`
3. **Verify**: Build clean; loaded track starts paused; `f`/`v`/`F`/`V` increment by 0.01
4. **Process**: Confirm ready to archive
