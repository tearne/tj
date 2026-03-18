# Verification

- Launching with no argument opens the file browser in the current working directory.
- Launching with a directory path opens the file browser rooted there.
- Launching with a valid file path plays the track and renders the TUI.
- Beat indicator flashes at the correct tempo, aligned with the audio.
- Phase offset adjustment shifts the flash timing immediately.
- Waveform overview renders the full track immediately after load.
- Detail view updates position in real time during playback without visible lag.
- Beat jump moves playback position by the correct number of beats at the detected BPM.
- All supported formats play without error.
- Quitting cleanly exits without errors or leftover terminal state.
