# Proposal: Default Key Mapping
**Status: Note**

## Intent
When `~/.config/tj/config.toml` does not exist, tj currently leaves all functions unbound. This proposal adds auto-creation of the file with a full default key mapping on first run, so tj works out of the box without manual configuration.

## Unresolved
- Should tj silently create the file, or prompt/notify the user?
- Should the default mapping exactly match the current dev config, or be reconsidered at this point?
