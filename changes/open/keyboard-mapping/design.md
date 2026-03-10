# Design: Keyboard Mapping
**Status: Draft**

## Approach

Add a `toml` crate dependency. Define an `Action` enum covering all 29 mappable functions. Implement `load_keymap()` to read `~/.config/tj/config.toml` and produce a `HashMap<KeyCode, Action>`. Refactor the player event loop to dispatch through that map instead of matching key codes directly. Hold actions (`nudge_backward`, `nudge_forward`) require separate handling on `Press`/`Repeat` vs `Release`.

The browser keeps its own hard-coded bindings — the proposal covers player controls only.

### `Action` enum

```rust
enum Action {
    PlayPause, Quit,
    BeatJumpBackward, BeatJumpForward,
    BeatUnit1, BeatUnit2, BeatUnit3, BeatUnit4, BeatUnit5, BeatUnit6, BeatUnit7,
    NudgeBackward, NudgeForward,
    OffsetIncrease, OffsetDecrease,
    ZoomIn, ZoomOut,
    HeightIncrease, HeightDecrease,
    VolumeUp, VolumeDown,
    BpmHalve, BpmDouble, BpmRedetect,
    PaletteCycle,
    OpenBrowser,
    Help,
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
- anything else → `None` (logged, binding skipped)

### `load_keymap()`

```rust
fn load_keymap() -> HashMap<KeyCode, Action>
```

1. Resolve `~/.config/tj/config.toml`. If the file is absent or unreadable, return an empty map.
2. Parse with `toml::from_str`. If parsing fails, print a warning and return an empty map.
3. Read the `[keys]` table. For each `function_name = "key_string"` entry:
   - Look up the function name in a static `&[(&str, Action)]` table.
   - Parse the key string via `parse_key`.
   - Insert into the map; skip and warn on unknown function names or unparseable keys.

### Event loop refactor

Replace the direct `match key.code { ... }` block with:

```rust
// hold actions — checked before keymap dispatch
match (key.kind, key.code) {
    (Press | Repeat, code) if keymap.get(&code) == Some(&Action::NudgeBackward) => { ... }
    (Press | Repeat, code) if keymap.get(&code) == Some(&Action::NudgeForward)  => { ... }
    (Release, code) if matches!(keymap.get(&code),
        Some(&Action::NudgeBackward) | Some(&Action::NudgeForward)) => { ... }
    _ => {}
}
// fire-on-press actions
if key.kind == Press || key.kind == Repeat {
    if let Some(action) = keymap.get(&key.code) {
        match action { ... }
    }
}
```

`NudgeBackward`/`NudgeForward` are excluded from the fire-on-press block — they are fully handled in the hold block.

### Dev config

Create `~/.config/tj/config.toml` with the full default binding table from the proposal. This is a one-time manual task documented here; no code auto-creates the file (deferred to `default-keymapping`).

## Tasks

1. **Impl**: Add `toml = "0.8"` to `Cargo.toml`
2. **Impl**: Define `Action` enum and static function-name lookup table
3. **Impl**: Implement `parse_key()` and `load_keymap()`
4. **Impl**: Refactor player event loop to dispatch via keymap; handle hold actions
5. **Process**: Create `~/.config/tj/config.toml` with full default bindings
6. **Verify**: Confirm all player functions respond to configured keys; confirm unbound functions have no effect
7. **Process**: Confirm ready to archive
