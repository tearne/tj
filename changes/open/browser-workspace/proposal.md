# Browser Workspace
**Type**: Proposal
**Status**: Draft

## Intent

The browser currently supports only directory-by-directory navigation. Finding a specific track requires knowing roughly where it lives in the filesystem. This change introduces fuzzy search across a nominated workspace directory, letting the user find tracks by typing part of their filename or path without leaving the browser.

## Specification Deltas

### ADDED

**SPEC/browser.md — Workspace**

A workspace is a directory nominated by the user as the root for fuzzy search. It is stored in the cache and persists across sessions.

- When no workspace has been set, a prompt is displayed at the top of the browser: `Press @ to set this directory as your search workspace`.
- Pressing `@` sets the current browsing directory as the workspace. The prompt is replaced by the search field (see below).
- Pressing `@` when a workspace is already set replaces it with the current browsing directory.

**SPEC/browser.md — Search field and results**

When a workspace is set, a search field is displayed at the top of the browser.

- Characters typed by the user are appended to the search term and displayed in the search field.
- When the search term is non-empty, the browser list is replaced with fuzzy-matched audio files found recursively under the workspace, each displayed with its path relative to the workspace root.
- Results are ordered by match quality (best match first).
- `↑` / `↓` navigate the results list. `Enter` loads and plays the selected file.
- `←` / `Backspace` navigates to the parent directory as normal, clearing the search term as a side effect.
- When the search term is empty, the browser shows the normal directory listing at the current path.

**SPEC/browser.md — Controls (additions)**

| Key | Action |
|-----|--------|
| `@` | Set current directory as workspace |
| (any printable character) | Append to search term (workspace required) |

**SPEC/cache.md — Workspace directory**

- `browser_workspace`: the absolute path of the nominated workspace directory. Absent if no workspace has been set. Saved immediately when `@` is pressed.

## Scope

- **In scope**: workspace nomination, fuzzy filename/path search, result display with relative paths, cache persistence of workspace.
- **Out of scope**: search across tag fields (title, artist, album); multiple workspaces; saving or naming searches; search result ordering beyond match quality.
