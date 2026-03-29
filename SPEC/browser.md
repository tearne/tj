# Browser

## Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move cursor |
| `Enter` | Navigate into directory / load and play audio file (or selected search result) |
| `←` / `Backspace` | Delete last search character (when searching); otherwise go to parent directory |
| `Esc` | Clear search term (when searching); otherwise return to player |
| `@` | Set current directory as workspace |
| `'` | Clear workspace |
| `#` | Preview: start playback of the highlighted audio file from 20% through the track; press `#` again to restart; any other key stops the preview |
| `q` | Quit (only when search term is empty) |
| any printable character (except `@`, `'`, `#`) | Append to search term (workspace required) |

## Behaviour

- Displays all files and subdirectories in the current directory, sorted alphabetically.
- Directories are visually distinguished (e.g. trailing `/`, different colour).
- Compatible audio files (FLAC, MP3, OGG, WAV, AAC, OPUS) are highlighted.
- Non-audio files are shown but cannot be selected or navigated into.
- A header shows the current directory path.
- Selecting an audio file dismisses the browser and begins playback.
- The browser can be opened and closed from the player at any time with `z`. Audio continues playing while the browser is open. Pressing `Esc` returns to the player view; selecting a new file loads and plays it.
- Pressing `#` on a highlighted audio file begins streaming playback from approximately 20% into the track (30 s offset if the file's duration is not available from the header). The preview plays through the main output independently of the deck players. Pressing `#` again restarts the preview from the same position. Any other keypress stops the preview; the key's normal action is then applied. Preview stops automatically when the browser is closed.
- Directories are not previewable; `#` is a no-op when the cursor is on a directory.
- If the target deck is playing when the browser key is pressed, an error is shown: `"Track is playing — open browser?  [y] open   [Esc/n] cancel"`. Pressing `y` within the 5-second window opens the browser; `Esc` or `n` cancels.
- The last visited directory is persisted to the cache between sessions. The browser always opens at the last visited path (falling back to CWD if it no longer exists). If a directory or file argument is given on the command line, it overrides the last visited path for the first browser open of that session only; subsequent opens resume from last visited.

## Workspace

A workspace is a directory nominated by the user as the root for fuzzy search. It is stored in the cache and persists across sessions. If the stored workspace directory no longer exists (e.g. removable media), the workspace is silently discarded and the user is prompted to set a new one.

- When no workspace has been set, a prompt is shown at the top of the browser: `Press ~ to set this directory as your search workspace`.
- Pressing `@` sets the current browsing directory as the workspace; the prompt is replaced by the search field.
- Pressing `@` when a workspace is already set replaces it with the current directory.
- Pressing `'` clears the workspace; the search field is replaced by the prompt and any active search is discarded.
- When a workspace is set, the browser title shows the workspace root path followed by the current directory's path relative to the workspace (dimmed). When no workspace is set, the title shows the full current path.

## Search

When a workspace is set, a search field is shown at the top of the browser.

- Characters typed by the user append to the search term and are shown in the search field.
- When the search term is non-empty, the browser list is replaced with fuzzy-matched audio files found recursively under the workspace, each displayed with its path relative to the workspace root.
- Results are ordered by match quality (best match first).
- `↑` / `↓` navigate the results list. `Enter` loads and plays the selected file.
- `Backspace` removes the last character from the search term; when the term becomes empty the directory listing is restored.
- `Esc` clears the search term entirely and restores the directory listing.
- When the search term is empty, the browser shows the normal directory listing at the current path.

## Constraints

- Compatible audio extensions: `flac`, `mp3`, `ogg`, `wav`, `aac`, `opus`, `m4a`.
