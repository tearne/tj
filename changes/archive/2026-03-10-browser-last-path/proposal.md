# Proposal: Browser Remembers Last Visited Path
**Status: Ready for Review**

## Intent
The file browser currently always opens at the directory of the playing file, or the current working directory. This proposal persists the last directory the user navigated to, so the browser reopens there across sessions.

## Specification Deltas

### ADDED
- The last directory visited in the file browser is persisted between sessions.
- On startup with no argument, the browser opens at the last visited path (falling back to the current working directory if the path no longer exists).
- On startup with a directory or file argument, the browser's initial root is set from that argument as normal — but only for the first browser open in that session. Subsequent opens (via `b`) resume from the last visited path as usual.
- The last visited path is updated whenever the browser navigates to a new directory.

### MODIFIED
- Last visited path is stored in the existing cache file (`~/.local/share/tj/cache.json`) as a top-level field.

### MODIFIED (continued)
- Pressing `b` mid-session always opens the browser at the last visited path, rather than the directory of the currently playing file.
