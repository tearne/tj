# Design: Remove BPM Halve/Double Controls
**Status: Draft**

## Approach

Remove all traces of `bpm_halve` / `bpm_double`: the action enum variants, ACTION_NAMES entries, config keys, key handlers, and the help overlay lines.

## Tasks

1. ✓ **Impl**: Remove `bpm_halve` / `bpm_double` from `resources/config.toml`.
2. ✓ **Impl**: Remove `BpmHalve` / `BpmDouble` from the `Action` enum, `ACTION_NAMES`, and their match arm handlers.
3. ✓ **Impl**: Remove `h` / `H` lines from the `[?]` help overlay string.
4. **Verify**: `h` and `H` are unbound and produce no effect. Config loads without error.
5. **Process**: Confirm ready to archive.
