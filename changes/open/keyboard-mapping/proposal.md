# Proposal: Keyboard Mapping
**Status: Approved**

## Intent
All player controls are currently hard-coded. A user-configurable keyboard mapping allows rebinding any control without recompiling.

## Specification Deltas

### ADDED
- **Keyboard mapping**: Key bindings are loaded from `~/.config/tj/config.toml` at startup under a `[keys]` table. Each entry maps a function name to a key string. If a function has no entry in the file, it has no binding and cannot be triggered. If the file is absent or the `[keys]` table is missing, all functions are unbound.
- **Mappable functions** (canonical list):

| Function | Default key (dev config) |
|----------|--------------------------|
| `play_pause` | `space` |
| `quit` | `esc` |
| `jump_forward_1` | `1` |
| `jump_backward_1` | `q` |
| `jump_forward_4` | `2` |
| `jump_backward_4` | `w` |
| `jump_forward_16` | `3` |
| `jump_backward_16` | `e` |
| `jump_forward_64` | `4` |
| `jump_backward_64` | `r` |
| `nudge_backward` | `,` |
| `nudge_forward` | `.` |
| `micro_jump_backward` | `c` |
| `micro_jump_forward` | `d` |
| `offset_increase` | `+` |
| `offset_decrease` | `-` |
| `zoom_in` | `Z` |
| `zoom_out` | `z` |
| `height_increase` | `}` |
| `height_decrease` | `{` |
| `volume_up` | `j` |
| `volume_down` | `m` |
| `bpm_halve` | `h` |
| `bpm_double` | `H` |
| `bpm_increase` | `f` |
| `bpm_decrease` | `v` |
| `bpm_redetect` | `t` |
| `palette_cycle` | `p` |
| `open_browser` | `b` |
| `help` | `?` |

- **Key string format**: single printable characters are written as-is (`q`, `[`, `+`). Special keys use lowercase names: `space`, `left`, `right`, `up`, `down`, `enter`, `backspace`, `esc`. Case-sensitive characters are written as-is (`H` vs `h`).
- **Hard-coded quit**: Ctrl-C always quits unconditionally and is not configurable via the keymap.
- **Hold actions**: `nudge_backward` and `nudge_forward` activate on key press and deactivate on release. All other actions fire on press.

### MODIFIED
- The hard-coded key bindings in the player event loop are replaced by a dispatch table derived from the config file.

## Scope
- **In scope**: loading and parsing the config; building the dispatch table; replacing hard-coded bindings; creating a working dev config at `~/.config/tj/config.toml`.
- **Out of scope**: auto-creating the config file with defaults when absent (separate proposal).
