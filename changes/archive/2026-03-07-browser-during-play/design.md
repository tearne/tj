# Design: Browser During Play
**Status: Ready for Review**

## Approach

### BrowserResult enum
`run_browser` currently returns `Option<PathBuf>` — `None` covers both "quit" and "return to player". Replace with an explicit enum:

```rust
enum BrowserResult {
    Selected(PathBuf),  // user picked a file
    ReturnToPlayer,     // Esc — go back to player
    Quit,               // q — exit the app
}
```

When no player is active (launch-time browser), `ReturnToPlayer` is treated as `Quit` in `main()`.

### tui_loop changes
- Receives `&mut Terminal` (already does).
- Returns `io::Result<Option<PathBuf>>` — `None` = quit, `Some(path)` = load this file next.
- On `b` key: call `run_browser` rooted at the current file's parent directory. The player is not touched — rodio runs on its own thread and audio continues uninterrupted while the browser occupies the TUI. Act on `BrowserResult`:
  - `ReturnToPlayer` → continue the loop; the player is already running.
  - `Selected(path)` → save current offset to cache; return `Ok(Some(path))`.
  - `Quit` → save current offset to cache; return `Ok(None)`.

### main() load loop
Extract the decode-and-play block into a `load_and_play` loop:

```
mut next_path = initial file path
loop:
    decode + BPM + waveform for next_path
    run tui_loop → Ok(None): break | Ok(Some(p)): next_path = p, continue
```

The old `Player` is simply dropped (rodio stops it) and a new one is created each iteration.

## Tasks
1. ✓ Impl: add `BrowserResult` enum; update `run_browser` to return `io::Result<BrowserResult>`, mapping `Esc` → `ReturnToPlayer` and `q` → `Quit`; update `main()` call site
2. ✓ Impl: update `tui_loop` to return `io::Result<Option<PathBuf>>`; handle `b` key → `run_browser` → act on result (no player stop needed)
3. ✓ Impl: wrap decode+play block in `main()` in a loop driven by `tui_loop`'s return value; update key hints in TUI to show `b`
4. ✓ Verify: browse → select track → plays; `b` mid-play → browse → `Esc` → resumes; `b` → select new track → plays; `q` from browser → quits
5. ✓ Process: confirm ready to archive
