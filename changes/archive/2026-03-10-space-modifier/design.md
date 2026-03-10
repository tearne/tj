# Design: Space as Modifier Key
**Status: Approved**

## Approach

### Key binding representation

Replace the keymap's `HashMap<KeyCode, Action>` key type with an enum:

```rust
#[derive(Hash, Eq, PartialEq)]
enum KeyBinding {
    Key(KeyCode),
    SpaceChord(KeyCode),
}
```

`parse_key` is updated to return `KeyBinding`:
- `"space+z"` → `KeyBinding::SpaceChord(KeyCode::Char('z'))`
- anything else → `KeyBinding::Key(...)` as before

`load_keymap` returns `HashMap<KeyBinding, Action>`.

### State

Add two booleans:
- `space_held: bool` — set on Space Press, cleared on Space Release.
- `space_chord_fired: bool` — set when a chord fires while Space is held, cleared on Space Press. Prevents a stale chord-fired flag carrying over.

### Event loop changes

Space Press/Release are handled in the all-key-kinds block (alongside nudge):
- `Press, Space` → `space_held = true; space_chord_fired = false`
- `Release, Space` → `space_held = false`

In the press-only dispatch, before the normal keymap lookup: if `space_held`, look up `KeyBinding::SpaceChord(key.code)`. If found, fire the action and set `space_chord_fired = true`; skip normal dispatch. If not found, fall through to normal dispatch (Space+unknown key acts as if Space is not held).

Space itself is not in the keymap as `KeyBinding::Key(Space)` — `play_pause` moves to `space+z`.

### New action

Add `Action::TempoReset` — sets `bpm = base_bpm`, calls `player.set_speed(1.0)`.

### Config

Update `resources/config.toml`:
- `play_pause = "space+z"`
- `tempo_reset = ["space+f", "space+v"]`

Add `("tempo_reset", Action::TempoReset)` to `ACTION_NAMES`.

### Help popup

Update `Space` line to `space+z` for play/pause; add `space+f/v` for tempo reset.

## Tasks

1. ✓ **Impl**: Add `KeyBinding` enum; update `parse_key` to return `KeyBinding` and handle `space+<key>` format; update `load_keymap` signature
2. ✓ **Impl**: Add `space_held`/`space_chord_fired` state; wire Space Press/Release in all-key-kinds block; update press dispatch to check `SpaceChord` first
3. ✓ **Impl**: Add `Action::TempoReset` handler; update `ACTION_NAMES`; update `resources/config.toml`; update help popup
4. ✓ **Verify**: Confirm `space+z` plays/pauses; confirm `space+f`/`space+v` resets tempo; confirm bare Space does nothing; confirm other keys unaffected
5. ✓ **Process**: Confirm ready to archive
