# Design: Default Key Mapping
**Status: Draft**

## Approach

Create `resources/config.toml` as the canonical default. Embed it in the binary with:

```rust
const DEFAULT_CONFIG: &str = include_str!("../resources/config.toml");
```

Update `load_keymap()` to handle the no-config case: if neither binary-adjacent nor user config exists, write `DEFAULT_CONFIG` to `~/.config/tj/config.toml` (creating the directory if needed), print a notice to stderr, then parse `DEFAULT_CONFIG` directly.

The parse path for the auto-created case skips the file read and uses `DEFAULT_CONFIG` directly — no need to re-read the file we just wrote.

## Tasks

1. ✓ **Impl**: Create `resources/config.toml` with the full default bindings (copy from `~/.config/tj/config.toml`)
2. ✓ **Impl**: Add `const DEFAULT_CONFIG: &str = include_str!("../resources/config.toml")` and update `load_keymap()` to auto-create and parse it when no config is found
3. **Verify**: Confirm removing `~/.config/tj/config.toml` causes auto-creation on next run with correct bindings; confirm notice is printed; confirm binary-adjacent config still takes precedence
4. **Process**: Confirm ready to archive
