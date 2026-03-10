# Design: Nudge Mode Toggle
**Status: Draft**

## Approach

Add a `NudgeMode` enum (`Jump` | `Warp`) and a `nudge_mode` state variable (default `Jump`).

**Toggle** (`C` / `D` on Press): flip `nudge_mode`. If switching away from `Warp` while a warp is active (nudge ≠ 0), reset `nudge = 0` and restore speed to `bpm / base_bpm`.

**`c`/`d` in `Jump` mode**: unchanged — `set_position` ±10ms on Press/Repeat.

**`c`/`d` in `Warp` mode**:
- `Press | Repeat`: set `nudge = ±1`, call `player.set_speed(bpm / base_bpm * (1.0 ± 0.1))`.
- `Release`: set `nudge = 0`, call `player.set_speed(bpm / base_bpm)`.
- While paused the existing `nudge != 0` branch in the render loop already drifts `smooth_display_samp` and calls `set_position` — no extra code needed.

**Remove `,`/`.`**: delete their Press/Repeat/Release handlers.

**Info bar**: add a `nudge:jump` / `nudge:warp` label. Placed after the zoom field, before `[?]`.

**Help popup**: remove `,`/`.` line; update `c`/`d` line to describe both modes; add `C`/`D` toggle line.

## Tasks

1. ✓ **Impl**: Add `NudgeMode` enum and `nudge_mode` state; implement `C`/`D` toggle handler (with active-warp reset); update `c`/`d` handlers to branch on mode (jump: existing; warp: set nudge + speed on press/repeat, reset on release); delete `,`/`.` handlers.
2. ✓ **Impl**: Update info bar to show nudge mode; update help popup.
3. **Verify**: Confirm toggle switches mode; confirm `jump` behaviour unchanged; confirm `warp` matches old `,`/`.`; confirm info bar updates; confirm `,`/`.` no longer respond.
4. **Process**: Confirm ready to archive.
