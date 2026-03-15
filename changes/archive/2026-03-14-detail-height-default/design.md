# Design: Reduce Default Detail Waveform Height
**Status: Draft**

## Approach

Two changes:

1. `detail_height` initialisation changed from `8` to `6` (already done).
2. `DisplayConfig` gains a `detail_height: usize` field (default `6`). `load_config` reads it from `[display]` in `config.toml`, falling back to `6` if absent. The `tui_loop` initialises its `detail_height` variable from `display_cfg.detail_height` instead of the hardcoded literal.

## Tasks

1. ✓ **Impl**: Change default `detail_height` from `8` to `6`
2. ✓ **Impl**: Add `detail_height` to `DisplayConfig`; read from `[display]` config; initialise `tui_loop` variable from config
3. **Verify**: Build clean; default height on launch is 4 waveform rows + 2 tick rows; setting `detail_height = 8` in config restores old behaviour
4. **Process**: Confirm ready to archive
