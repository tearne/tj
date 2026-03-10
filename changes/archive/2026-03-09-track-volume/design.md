# Design: Track Volume Control
**Status: Approved**

## Approach

`Player::set_volume(f32)` is available in rodio 0.22. Volume is stored as a local `f32` in `[0.0, 1.0]`, defaulting to `1.0` (100%). `↑` increases by 0.05, `↓` decreases by 0.05, clamped to `[0.0, 1.0]`. `player.set_volume()` is called immediately on each change.

`↑`/`↓` are currently unused in the player (they are used in the browser, which handles events independently). No conflicts.

Volume is displayed in the info bar as `vol: 80%` (dim text, same style as other fields). Not persisted.

## Tasks
1. ✓ Impl: Add `volume: f32 = 1.0` state; handle `↑`/`↓` keys; call `player.set_volume()`
2. ✓ Impl: Show `vol: N%` in info bar; add to help popup
3. ✓ Verify: volume changes audibly with `↑`/`↓`; info bar reflects current level; clamps at 0% and 100%
4. ✓ Process: confirm ready to archive
