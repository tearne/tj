# Rename to deck
**Type**: Proposal
**Status**: Archived

## Intent
The application name `tj` is opaque. `deck` is the core domain concept and makes the binary self-describing.

## Specification Deltas

### MODIFIED
- Binary name: `tj` → `deck`
- Config directory: `~/.config/tj/` → `~/.config/deck/`
- Cache/data directory: `~/.local/share/tj/` → `~/.local/share/deck/`
- All user-visible strings (window titles, diagnostic messages) updated to match

## Scope
- **In scope**: binary name, data paths, user-visible strings
- **Out of scope**: project directory name, SPEC files, migration code (no users yet)

