# Proposal: Default Key Mapping
**Status: Draft**

## Intent
When no `config.toml` is found, tj currently leaves all functions unbound. This proposal ensures tj works out of the box by:
1. Shipping a canonical `config.toml` in a `resources/` directory alongside the source.
2. Auto-creating `~/.config/tj/config.toml` from that file on first run if neither the binary-adjacent nor the user config exists.

## Specification Deltas

### ADDED
- A `resources/config.toml` file in the repository serves as the canonical default key mapping and is embedded in the binary at compile time via `include_str!`.
- On startup, if no config file is found (neither adjacent to the binary nor at `~/.config/tj/config.toml`), tj writes the embedded default to `~/.config/tj/config.toml`, creating the directory if necessary, and loads it. A one-line notice is printed to stderr: `tj: created default config at ~/.config/tj/config.toml`.
- The `resources/` directory is added to the repository.

### MODIFIED
- The config search order becomes: (1) binary-adjacent `config.toml`, (2) `~/.config/tj/config.toml`, (3) auto-create from embedded default and load.

## Scope
- **In scope**: `resources/config.toml`; embedding via `include_str!`; auto-create logic.
- **Out of scope**: a full settings UI; merging user config with defaults when keys are missing.
