# Proposal: Browser During Play
**Status: Ready for Review**

## Intent
Allow the user to open the file browser while a track is playing, browse and select a new track, and have the transition happen seamlessly — the current track keeps playing through loading and analysis of the next one.

## Specification Deltas

### ADDED

**Returning to the browser from the player**:
- Pressing `b` in the player view opens the file browser, rooted at the directory containing the currently playing file.
- Audio continues playing while the browser is open.
- Pressing `Esc` in the browser (without selecting a file) returns to the player view.
- Pressing `q` in the browser quits the application.

**Track transition**:
- Selecting an audio file in the browser shows a loading indicator while the new track is decoded and analysed. Audio from the current track continues playing during this period.
- Once the new track is ready, the current track stops and the new track begins playing immediately. The player view is displayed.

### MODIFIED

- **Player Controls** table: add `b` → "Open file browser".
- **File Browser Controls** table: add `Esc` → "Return to player (if one is active)".
