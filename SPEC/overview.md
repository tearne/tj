# Overview

A terminal-based two-deck music player written in Rust, with real-time waveform visualisation and beat-aware transport controls. Each deck is an independent playback unit; a single active deck receives transport input at any time.

## Launching
```
tj [path]
```
- If `path` is an audio file, opens and begins playing it immediately.
- If `path` is a directory, opens the file browser rooted at that directory.
- If `path` is omitted, the player opens with an empty deck; a startup notification on the notification bar prompts the user to press `z` to open the file browser.

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
