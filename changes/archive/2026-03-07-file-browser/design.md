# Design: File Browser
**Status: Ready for Review**

## Approach

### Argument handling
`main()` parses `args[1]` as an optional path. If it resolves to a directory (or is absent), the browser runs first and returns a `PathBuf`. If it resolves to a file, the existing load path runs immediately. The alternate screen is entered once and shared across both modes.

### Browser state
A `BrowserState` struct holds:
- `cwd: PathBuf` — current directory
- `entries: Vec<DirEntry>` — sorted alphabetically by name, dirs and files intermixed
- `cursor: usize` — currently highlighted row
- `scroll: usize` — top-of-viewport index (for long directories)

`BrowserState::read_dir()` reads the directory, sorts entries, and resets cursor/scroll. Parent dir (`..`) is prepended as a synthetic entry when not at the filesystem root.

### Audio detection
A const `AUDIO_EXTENSIONS: &[&str]` lists the supported extensions (`flac`, `mp3`, `ogg`, `wav`, `aac`, `opus`, `m4a`). A helper `is_audio(path)` checks the extension. Non-audio, non-directory entries are shown but skipped when navigating (cursor jumps over them) and ignored on Enter.

### Browser TUI loop
`run_browser(terminal, start_dir) -> io::Result<Option<PathBuf>>`

- Renders a full-screen block titled with the current path.
- Entries are rendered as a scrollable list. Each row is styled:
  - Directory: Yellow
  - Audio file: Green (bright)
  - Other file: DarkGray
- Cursor row is highlighted (reversed style).
- Keys:
  - `Up`/`Down` — move cursor, skipping non-audio files (only dirs and audio files are cursor-stoppable)
  - `Enter` — navigate into dir, or return `Ok(Some(path))` for audio file
  - `Backspace` / `Left` — go to parent directory
  - `q` / `Esc` — return `Ok(None)` (quit)
- Returns `Ok(Some(path))` when an audio file is selected; `Ok(None)` on quit.

### main() flow
```
args[1] -> optional path
  absent or directory -> run_browser -> Option<PathBuf>
    None -> exit cleanly
    Some(path) -> load and play
  file -> load and play directly
```

The alternate screen / raw mode wraps both browser and player in a single setup/teardown, so there is no flash between modes.

## Tasks
1. ✓ Tests: unit tests for `is_audio` helper and `BrowserState` sorting/navigation logic (no TUI, pure logic)
2. ✓ Impl: add `AUDIO_EXTENSIONS` + `is_audio` + `BrowserState` (read_dir, cursor nav)
3. ✓ Impl: add `run_browser` TUI loop
4. ✓ Impl: update `main()` — optional arg, shared terminal setup, browser-then-player flow
5. ✓ Verify: build and smoke-test: launch with no arg, with dir arg, with file arg; navigate dirs; select a track
6. ✓ Process: confirm ready to archive
