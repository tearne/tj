# Design: Keyboard Mapping
**Status: Draft**

## Approach

Add a `toml` crate dependency. Define an `Action` enum covering all mappable functions. Implement `load_keymap()` to read `~/.config/tj/config.toml` and produce a `HashMap<KeyCode, Action>`. Refactor the player event loop to dispatch through that map.

The browser keeps its own hard-coded bindings — this change covers player controls only.

### `Action` enum

```rust
enum Action {
    PlayPause, Quit,
    JumpForward1, JumpForward4, JumpForward16, JumpForward64,
    JumpBackward1, JumpBackward4, JumpBackward16, JumpBackward64,
    NudgeBackward, NudgeForward, NudgeModeToggle,
    OffsetIncrease, OffsetDecrease,
    ZoomIn, ZoomOut,
    HeightIncrease, HeightDecrease,
    VolumeUp, VolumeDown,
    BpmHalve, BpmDouble, BpmIncrease, BpmDecrease, BpmRedetect,
    PaletteCycle, OpenBrowser, Help,
}
```

### Key string parsing

`parse_key(s: &str) -> Option<KeyCode>`:
- `"space"` → `KeyCode::Char(' ')`
- `"left"` / `"right"` / `"up"` / `"down"` → `KeyCode::Left` etc.
- `"enter"` → `KeyCode::Enter`
- `"backspace"` → `KeyCode::Backspace`
- `"esc"` → `KeyCode::Esc`
- single char `c` → `KeyCode::Char(c)`
- anything else → `None` (warning printed, binding skipped)

### `load_keymap()`

```rust
fn load_keymap() -> HashMap<KeyCode, Action>
```

1. Resolve `~/.config/tj/config.toml`. If absent or unreadable, return an empty map.
2. Parse with `toml::from_str`. If parsing fails, print a warning and return an empty map.
3. Read the `[keys]` table. For each `function_name = "key_string"` entry:
   - Look up the function name in a static `&[(&str, Action)]` table.
   - Parse the key string via `parse_key`.
   - Insert into the map; skip and warn on unknown function names or unparseable keys.

### Event loop refactor

Replace the existing direct `match key.code { ... }` and nudge blocks with:

```rust
// NudgeBackward/Forward: handled for all key kinds (press, repeat, release).
// Behaviour depends on nudge_mode.
match (key.kind, key.code) {
    (Press | Repeat, code) if keymap.get(&code) == Some(&Action::NudgeBackward) => {
        match nudge_mode {
            NudgeMode::Jump => { /* seek -10ms */ }
            NudgeMode::Warp => { nudge = -1; player.set_speed(...); }
        }
    }
    (Press | Repeat, code) if keymap.get(&code) == Some(&Action::NudgeForward) => {
        match nudge_mode {
            NudgeMode::Jump => { /* seek +10ms */ }
            NudgeMode::Warp => { nudge = 1; player.set_speed(...); }
        }
    }
    (Release, code) if matches!(keymap.get(&code),
        Some(&Action::NudgeBackward) | Some(&Action::NudgeForward)) =>
    {
        if nudge_mode == NudgeMode::Warp {
            nudge = 0; player.set_speed(bpm / base_bpm);
        }
    }
    _ => {}
}

// All other actions: press only.
if key.kind == Press {
    match keymap.get(&key.code) {
        Some(Action::NudgeModeToggle) => { /* toggle nudge_mode, reset if active */ }
        Some(action) => { /* existing match arms */ }
        None => {}
    }
}
```

`NudgeBackward`/`NudgeForward` are excluded from the press-only block.

Note: `JumpForward*`/`JumpBackward*` currently fire on `Press` only (already guarded). `NudgeBackward`/`NudgeForward` in `Jump` mode use `Press | Repeat`; in `Warp` mode only `Press` is needed (speed is held until `Release`).

### Dev config

Create `~/.config/tj/config.toml` with the full default binding table from the proposal.

## Tasks

1. ✓ **Impl**: Add `toml = "0.8"` to `Cargo.toml`
2. ✓ **Impl**: Define `Action` enum and static function-name lookup table
3. ✓ **Impl**: Implement `parse_key()` and `load_keymap()`
4. ✓ **Impl**: Refactor player event loop to dispatch via keymap
5. ✓ **Process**: Create `~/.config/tj/config.toml` with full default bindings
6. **Verify**: Confirm all player functions respond to configured keys; confirm unbound functions have no effect; confirm rebinding works
7. **Process**: Confirm ready to archive
