# Design: Consolidate Data Directory
**Status**: Approved

## Approach

Replace the two `~/.local/share/deck/` path strings with `~/.config/deck/` in `src/cache/mod.rs` and `src/main.rs`. No logic changes.

## Tasks
1. ✓ Impl: `src/cache/mod.rs` — update cache path
2. ✓ Impl: `src/main.rs` — update panic log path
3. ✓ Impl: `SPEC/cache.md`, `SPEC/deck.md` — update paths
4. ✓ Verify: `cargo build --release` clean
5. ✓ Process: confirm ready to archive
