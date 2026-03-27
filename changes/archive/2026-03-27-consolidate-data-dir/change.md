# Consolidate Data Directory
**Type**: Proposal
**Status**: Implementing

## Intent

Config and cache currently live in separate XDG directories (`~/.config/deck/` and `~/.local/share/deck/`). For a simple CLI tool this adds friction: two directories to locate, back up, or migrate. Consolidating both under `~/.config/deck/` gives a single, obvious location with no meaningful loss.

## Specification Deltas

### MODIFIED
- Cache path: `~/.local/share/deck/cache.json` → `~/.config/deck/cache.json`
- Panic log path: `~/.local/share/deck/panic.log` → `~/.config/deck/panic.log`

## Scope
- **In scope**: data paths in source and SPEC
- **Out of scope**: migration code (no users yet)
