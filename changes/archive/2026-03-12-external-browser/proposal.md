# Proposal: External File Browser Integration
**Status: Cancelled**

## Intent
`tj` ships its own minimal file browser. A dedicated TUI file browser such as Yazi provides a far richer experience (preview, bookmarks, search, bulk selection, mouse support, extensibility) essentially for free. If `tj` can delegate file selection to an external browser, the built-in browser can be retained as a fallback only, reducing the surface area that needs to evolve with the multi-deck work.

## Specification Deltas

### ADDED
- When an external file browser command is configured, activating the browser (`z`) suspends `tj`'s terminal, launches the external browser, and resumes `tj` with the chosen path loaded (if any).
- The external browser command is user-configurable via `config.toml`. The placeholder `{file}` in the command is replaced at runtime with a temporary file path; the external tool is expected to write the chosen path to that file.
- If the external browser command is configured but the binary is not found, `tj` falls back to the built-in browser and logs a warning to the info bar.
- If no external browser is configured, behaviour is unchanged: the built-in browser runs as today.
- Playback is paused before the external browser is launched and may optionally resume on return (see Behaviour).

### MODIFIED
- The built-in browser is retained as the default fallback; no existing behaviour changes when no external browser is configured.

## Behaviour

### Terminal suspend/resume sequence
1. Pause playback (if playing).
2. Tear down terminal: `disable_raw_mode`, `PopKeyboardEnhancementFlags`, `DisableMouseCapture`, `LeaveAlternateScreen`.
3. Write a temporary file path; spawn the configured command (blocking `wait()`).
4. Read the chosen path from the temp file (if written).
5. Re-initialise terminal: `enable_raw_mode`, `EnterAlternateScreen`, `EnableMouseCapture`, `PushKeyboardEnhancementFlags`; call `terminal.clear()`.
6. If a path was returned and is a valid audio file, load it (same flow as a browser selection today).
7. Resume playback if it was playing before step 1, unless a new track was loaded.

### Playback on return
If a new track was selected, playback starts from the beginning (consistent with current browser behaviour). If the user dismissed without selecting, playback resumes from the paused position.

### Example configurations
```toml
# Yazi (recommended)
browser_command = "yazi --chooser-file {file}"

# lf
browser_command = "lf -selection-path {file}"

# ranger
browser_command = "ranger --choosefile {file}"

# fzf (non-TUI, launches in the terminal inline)
browser_command = "fzf --filter '' < /dev/null | fzf > {file}"
```

## Scope
- **In scope**: single file selection via external browser; suspend/resume terminal lifecycle; fallback to built-in browser.
- **Out of scope**: directory navigation, playlist population, multi-file selection. These are deferred to the playlists proposal and any future multi-deck work.

## Conclusion

Cancelled. The implementation is straightforward (~50 lines, all patterns already present in the codebase), but the timing is poor: the multi-deck restructure is imminent and any code added now must survive or be reworked through it. The feature also benefits a narrow audience (users with Yazi or similar installed) and the built-in browser is adequate for current needs.

Alternative in-process widgets (`tui-file-explorer`, `rat-widget`) were evaluated. `tui-file-explorer` is the most promising but is two weeks old at time of review; `rat-widget` carries an excessive dependency tree for a file picker. Neither is preferable to the built-in browser at this time.

This proposal can be reopened post-multi-deck if the built-in browser has grown to feel limiting.
