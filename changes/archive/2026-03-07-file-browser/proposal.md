# Proposal: File Browser
**Status: Ready for Review**

## Intent
Remove the requirement to pass an audio file as a command-line argument. Instead, launch a navigable file browser when no file (or a directory path) is given, allowing the user to select a track from within the TUI.

## Specification Deltas

### ADDED

**File Browser behaviour**:
- When `tj` is launched with no argument, or with a directory path, a file browser is displayed rooted at that directory (defaulting to CWD).
- The browser lists all files and subdirectories, sorted alphabetically (dirs and files intermixed).
- Directories are visually distinguished (e.g. trailing `/` or different colour).
- Audio files (FLAC, MP3, OGG, WAV, AAC, OPUS) are highlighted to indicate they are selectable.
- Non-audio files are shown but cannot be selected or navigated into.
- Navigation:
  - `Up`/`Down` arrow keys move the cursor.
  - `Enter` on a directory navigates into it.
  - `Backspace` (or `Left`) navigates to the parent directory.
  - `Enter` on a compatible audio file loads and begins playing the track; the browser is replaced by the player view.
  - `q` quits the application.
- A header row shows the current directory path.

### MODIFIED

- **Usage**: `tj [path]` — `path` is optional. If omitted or a directory, the file browser opens. If a file, playback begins immediately (existing behaviour).

### REMOVED

- "Directory browser and playlist support" removed from Out of Scope.
