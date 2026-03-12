# Proposal: Filter Key Conflicts
**Status: Note**

## Observed
- `Ctrl+[` quits (terminal interprets it as `Escape`)
- `Alt+[` moves the playhead back (terminal interprets it as part of an escape sequence)

Both are side effects of using `[` and `]` for filter decrease/increase — certain modifier combinations alias to control characters or escape sequences that crossterm intercepts differently.

## Unresolved
- Is this a crossterm limitation or a config/key-routing issue?
- Should `[`/`]` be remapped to avoid these conflicts, or is the behaviour acceptable given that Ctrl/Alt usage is uncommon?
