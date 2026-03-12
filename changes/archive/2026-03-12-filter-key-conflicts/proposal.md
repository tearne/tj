# Proposal: Filter Key Conflicts
**Status: Ready for Review**

## Intent
Remap filter keys away from `[`/`]` to avoid terminal aliasing conflicts (`Ctrl+[` → Escape, `Alt+[` → escape sequence).

## Specification Deltas

### MODIFIED
- Filter decrease: `,` (was `[`)
- Filter increase: `.` (was `]`)
- Filter reset: `space+,` or `space+.` (was `space+[` / `space+]`)
- Config keys `filter_decrease`, `filter_increase`, `filter_reset` updated accordingly.
