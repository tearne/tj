# Design: Rename to deck
**Status**: Approved

## Approach
Mechanical find-and-replace of `tj` with `deck` across the crate name, data paths, window titles, and diagnostic message prefixes. No logic changes.

## Tasks
1. ✓ Impl: `Cargo.toml` — rename crate
2. ✓ Impl: `src/config/mod.rs` — update config path and `eprintln!` prefixes
3. ✓ Impl: `src/cache/mod.rs` — update cache path
4. ✓ Impl: `src/main.rs` — update window title and panic log path
5. ✓ Impl: `src/browser/mod.rs` — update browser window title
6. ✓ Impl: Update SPEC files (`config.md`, `cache.md`, `deck.md`, `overview.md`) to reflect new name and paths
7. ✓ Verify: `cargo build --release` clean
8. ✓ Process: confirm ready to archive
