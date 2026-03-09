# Proposal: Keyboard Mapping
**Status: Note**

## Intent
All player controls are currently hard-coded. A user-configurable keyboard mapping would allow rebinding any control without recompiling, and provide a canonical list of all mappable UI functions.

## Unresolved

### Configuration format
Three realistic options:

**TOML**
- Pros: widely known, simple key-value structure, good Rust library support (`toml` crate), human-readable, no programming knowledge required.
- Cons: no scripting or logic; complex conditional bindings are not expressible.
- Best for: simple rebinding with no dynamic behaviour.

**KDL**
- Pros: clean node-based syntax, growing ecosystem, expressive for nested config.
- Cons: less widely known than TOML, smaller Rust ecosystem, unfamiliar to most users.
- Best for: richer config structures where nesting is natural.

**Lua**
- Pros: fully programmable, could support conditional bindings and custom logic; familiar to power users from tools like Neovim.
- Cons: requires embedding a Lua runtime; significantly more complex to implement and maintain; overkill if only key rebinding is needed.
- Best for: extensible plugin-style configuration.

**Recommendation**: TOML for initial implementation. It covers all straightforward rebinding needs with minimal complexity. Lua can be reconsidered if scripting requirements emerge.

### Other open questions
- Where does the config file live? (`~/.config/tj/keys.toml` or similar XDG path.)
- What is the fallback when a key has no binding (warn silently or show in UI)?
- Should the config list all functions explicitly, or only overrides from the default map?
- The spec should enumerate all mappable UI functions as a canonical list.

## Specification Deltas (provisional)

### ADDED
- **Keyboard mapping**: Key bindings are configurable via a TOML file at `~/.config/tj/keys.toml`. The file maps function names to key combinations. Unrecognised keys are ignored; unbound functions use their default binding. The spec enumerates all mappable functions.
- **Mappable functions** (canonical list — to be finalised): `play_pause`, `quit`, `seek_backward`, `seek_forward`, `beat_jump_backward`, `beat_jump_forward`, `beat_unit_1` through `beat_unit_7`, `zoom_in`, `zoom_out`, `height_increase`, `height_decrease`, `offset_increase`, `offset_decrease`, `bpm_halve`, `bpm_double`, `bpm_redetect`, `open_browser`, `nudge_forward`, `nudge_backward`, `volume_up`, `volume_down`.
