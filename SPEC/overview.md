# Overview

A terminal-based two-deck music player written in Rust, with real-time waveform visualisation and beat-aware transport controls. Each deck is an independent playback unit; a single active deck receives transport input at any time.

## Launching
```
tj [path]
```
- If `path` is an audio file, opens and begins playing it immediately.
- If `path` is a directory, opens the file browser rooted at that directory.
- If `path` is omitted, the player opens with an empty deck; a startup notification on the notification bar prompts the user to press `z` to open the file browser.

## File Browser Controls
| Key | Action |
|-----|--------|
| `↑` / `↓` | Move cursor (skips non-audio files) |
| `Enter` | Navigate into directory / load and play audio file |
| `←` / `Backspace` | Go to parent directory |
| `Esc` | Return to player (if one is active) |
| `q` | Quit |

## File Browser Behaviour
- Displays all files and subdirectories in the current directory, sorted alphabetically.
- Directories are visually distinguished (e.g. trailing `/`, different colour).
- Compatible audio files (FLAC, MP3, OGG, WAV, AAC, OPUS) are highlighted.
- Non-audio files are shown but cannot be selected or navigated into.
- A header shows the current directory path.
- Selecting an audio file dismisses the browser and begins playback.
- The browser can be opened and closed from the player at any time with `z`. Audio continues playing while the browser is open. Pressing `Esc` returns to the player view; selecting a new file loads and plays it.
- The last visited directory is persisted to the cache between sessions. The browser always opens at the last visited path (falling back to CWD if it no longer exists). If a directory or file argument is given on the command line, it overrides the last visited path for the first browser open of that session only; subsequent opens resume from last visited.

## Constraints
- Implementation language: Rust.
- TUI framework: `ratatui`.
- Audio decoding: `symphonia`.
- Audio playback: `rodio`.
- BPM detection: `stratum-dsp`.
- Target platform: Linux (primary); other Unix-like systems are a stretch goal.

## Out of Scope (deferred)
- Cover art display.
- Shuffle, repeat.
- Multiple file / queue management.
