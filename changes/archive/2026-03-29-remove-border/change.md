# Remove Border

## Intent

The application renders inside a full-terminal border (`Borders::ALL`) that costs 2 rows and 2 columns. Removing it recovers that space for content. The version string currently in the border title moves to the right side of the global info bar.

## Approach

- Remove the outer `Block` — `inner` becomes `area` directly, recovering 2 rows and 2 columns
- Version string shown right-aligned on the global bar in the idle state only (no error, no notification); error and notification states are unaffected
- Idle state renders two spans: left-aligned `  {browser_dir}`, right-aligned version padded to fill the row width
- Affected files: `src/main.rs` only — the idle branch of the global status bar block
- No changes to `src/render/mod.rs` or any other layout

## Plan

- [x] UPDATE IMPL: remove outer `Block` and use `area` directly in place of `inner`
- [x] UPDATE IMPL: add right-aligned version span to the idle branch of the global status bar

## Conclusion

Removed the outer `Borders::ALL` block, recovering 2 rows and 2 columns. Version string moved to the right side of the global status bar, visible in the idle state only. Released as 0.6.25.
