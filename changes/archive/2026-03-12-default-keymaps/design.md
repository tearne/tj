# Design: Update Default Key Mappings
**Status: Draft**

## Approach

### Key binding changes (config.toml)
- `zoom_in` / `zoom_out`: `=` / `-`
- `offset_increase` / `offset_decrease`: `+` / `_`
- `open_browser`: `z`
- `level_up` / `level_down`: `j` / `m`
- `level_max` / `level_min`: `space+j` / `space+m` (new)
- `terminal_refresh`: `` ` `` (new)
- Remove `bpm_redetect`

### New actions
- `LevelMax` / `LevelMin`: set `volume = 1.0` / `volume = 0.0`, call `player.set_volume()`
- `TerminalRefresh`: call `terminal.clear()` then set a flag to force full redraw next frame

### Remove bpm_redetect
- Remove `BpmRedetect` from `Action` enum and `ACTION_NAMES`
- Remove its match arm handler (the mode-cycling re-detection logic including `detection_mode` state if present)
- Keep the tap-triggered background re-detection (the `b` follow-up) untouched

### Spec update
- Document that the `b` tap follow-up uses legacy autocorrelation constrained to ±5% of tapped BPM
- Remove `t` from the spec

## Tasks

1. **Impl**: Update `resources/config.toml` with all new/changed/removed bindings.
2. **Impl**: Add `LevelMax`, `LevelMin`, `TerminalRefresh` to `Action` enum and `ACTION_NAMES`.
3. **Impl**: Add handlers for `LevelMax`, `LevelMin`, `TerminalRefresh`.
4. **Impl**: Remove `BpmRedetect` action, `ACTION_NAMES` entry, handler, and associated re-detection mode cycling code.
5. **Verify**: All new bindings work. `t` is unbound. Terminal refresh clears glitches. Level max/min jump correctly.
6. **Process**: Confirm ready to archive.
