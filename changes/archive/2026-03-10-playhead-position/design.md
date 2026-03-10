# Design: Configurable Playhead Position
**Status: Draft**

## Approach

### Config file
Add a `[display]` section to `resources/config.toml`:
```toml
[display]
playhead_position = 20   # 0–100, % from left edge of detail panel
```

### Config loading refactor
`load_keymap()` currently owns the find-or-create logic and returns only the keymap. Refactor into:

- `resolve_config() -> String` — finds or creates the config file (same logic as today, no patching of existing files), returns the text.
- `load_config() -> (HashMap<KeyBinding, Action>, DisplayConfig)` — calls `resolve_config()` once, then passes the text to both `parse_keymap()` and `parse_display_config()`.

`load_keymap()` is replaced by `load_config()` at the call site in `run_player()`.

### DisplayConfig
```rust
struct DisplayConfig {
    playhead_position: u8,  // clamped to 0–100 after parse
}

impl Default for DisplayConfig {
    fn default() -> Self { Self { playhead_position: 20 } }
}
```

`parse_display_config(text: &str) -> DisplayConfig` reads `display.playhead_position` from the TOML value, clamps to 0–100, falls back to default on any error.

### centre_col update
Replace `let centre_col = dw / 2;` with:
```rust
let centre_col = ((dw as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
    .clamp(0, dw.saturating_sub(1));
```
`centre_col` is used in two places in the detail render block: `view_start` computation and the playhead marker column. Both pick up the change automatically.

## Tasks

1. ✓ **Impl**: Add `[display]` section with `playhead_position = 20` to `resources/config.toml`
2. ✓ **Impl**: Refactor `load_keymap()` → `resolve_config()` + `load_config()`; add `DisplayConfig` + `parse_display_config()`; update call site
3. ✓ **Impl**: Replace `centre_col = dw / 2` with the configured value; update the adjacent comment
4. ✓ **Verify**: Playhead renders at ~20% from left; out-of-range values clamp; `[keys]`-only configs fall back to default; no modifications made to existing config files
5. ✓ **Process**: Archive
