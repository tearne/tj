# Proposal: Playlists
**Status: Ready for Review**

## Intent
Allow the user to organise tracks into playlists, see where they are within a playlist, and move to the next or previous track without re-opening the file browser. Playlists are not auto-queued — the user controls when to advance — making the feature additive rather than changing existing single-track behaviour.

## Specification Deltas

### ADDED
- Playlist files in M3U8 format (UTF-8 `.m3u8` or plain `.m3u`) are recognised as a loadable type in the file browser alongside audio files.
- Loading a playlist file opens all listed tracks as an ordered playlist and begins playing the first track.
- While a playlist is active, an indicator in the info bar shows the current position and total (e.g. `[2/7]`).
- `Space+]` loads and plays the next track in the playlist; `Space+[` loads and plays the previous track. Reaching either end wraps.
- The active playlist persists across track loads within the session (advancing or retreating through it does not clear it). Loading a new audio file directly (via the browser) clears the active playlist.
- While a playlist is active, the file browser highlights the currently playing track and all other playlist members with a distinct indicator.

#### Playlist editor view
- `Space+P` opens an inline playlist editor panel, overlaid on the player view (audio continues playing).
- The editor lists all tracks in the current playlist, with the active track highlighted.
- Controls within the editor:
  - `↑` / `↓` — move cursor
  - `Enter` — load and play the selected track immediately
  - `d` — remove the track under the cursor from the playlist
  - `J` / `K` — move the track under the cursor down / up (reorder)
  - `a` — open the file browser in add-to-playlist mode; selecting a file appends it after the cursor position
  - `s` — save the current playlist back to its source file (or prompt for a path if it was created in-session)
  - `Esc` / `Space+P` — close the editor

#### Creating a new playlist
- From the file browser, `n` creates a new empty playlist and immediately opens it in the editor. The user can then add tracks, reorder, and save.

### MODIFIED
- The file browser recognises `.m3u` and `.m3u8` files as loadable (currently only audio files are loadable).
- The info bar gains a playlist position indicator `[N/T]` when a playlist is active.
- `Space+[` and `Space+]` are added as player key bindings for previous/next track.
- `Space+P` opens the playlist editor from the player view.

## Scope
- **In scope**: M3U/M3U8 read and write, single flat playlist, manual advance/retreat, inline editor with add/remove/reorder/save.
- **Out of scope**: nested playlists, shuffle, repeat, auto-advance on track end, playlist history, streaming URLs in playlists.
