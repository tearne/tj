# Browser Workspace

## Intent

Finding a specific track requires knowing roughly where it lives in the filesystem. This change introduces a nominated workspace directory and fuzzy search across it, so users can locate tracks by typing part of a filename or path without navigating the directory tree.

## Approach

**Workspace and fuzzy search** — add `browser_workspace: Option<PathBuf>` to the cache. `@` sets the current browsing directory as the workspace (replacing any previous). When a workspace is set, a search field replaces the workspace prompt at the top of the browser; typed characters filter the directory listing to fuzzy-matched audio files found recursively under the workspace, ordered by match quality.

**Fuzzy matching** — use `fuzzy-matcher` crate (SkimMatcherV2) for filename/path scoring. Walk the workspace tree once on first search keystroke, cache the file list in `BrowserState`; re-walk if the workspace changes.

**SPEC updates** — deck labels A/B → 1/2 throughout; art area row described as "Browser / cover art"; `browser_workspace` added to `SPEC/cache.md`; search field and `@` key added to `SPEC/browser.md`.

**Version bump** — patch.

## Plan

- [x] ADD `browser_workspace: Option<PathBuf>` to cache struct and serialisation
- [x] ADD `@` key in browser: sets workspace; prompt shown when no workspace set
- [x] UPDATE `BrowserState`: add `search_term: String` and `search_results: Option<Vec<PathBuf>>`
- [x] ADD fuzzy walk: `fuzzy-matcher` crate; walk workspace tree on first keystroke; rank by SkimMatcherV2 score
- [x] UPDATE `render_browser`: search field when workspace set; results list when term non-empty
- [x] UPDATE `handle_browser_key`: printable chars append to search term; `Backspace` clears term or navigates up if term empty
- [x] UPDATE SPEC/browser.md: workspace section, search field section, `@` key in controls table
- [x] UPDATE SPEC/cache.md: `browser_workspace` field
- [x] UPDATE version bump

## Conclusion

Workspace nomination (`@` to set, `'` to clear) and fuzzy search across the workspace tree are fully implemented. The browser is embedded in the art area of the main TUI. The title shows `@: workspace` in yellow with the bracketed relative cwd dimmed alongside; the deck indicator is yellow. The cached workspace is silently discarded on load if the directory no longer exists. SPEC/browser.md and SPEC/cache.md updated throughout.
