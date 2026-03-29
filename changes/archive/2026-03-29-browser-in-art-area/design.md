# Design: Browser in Art Area
**Status**: Approved

## Approach

`run_browser` currently owns its own draw/event loop, taking over the terminal entirely. To render the browser inline, browser state and key handling must move into the main loop. This also simplifies the architecture — one loop, one draw call.

The fullscreen path is eliminated: the browser always renders within the main draw call, using the art area when tall enough (≥ 8 rows) and the full terminal area otherwise.

## Tasks

- [ ] Add `browser_state: Option<(BrowserState, usize)>` to `tui_loop` — `usize` is the target deck slot (0 or 1)
- [ ] Replace `run_browser` call in `Deck1OpenBrowser`/`Deck2OpenBrowser` handlers with: set `browser_state`, skip the deferred-stop logic that currently wraps `run_browser`
- [ ] Extract `render_browser(frame, area, state)` from `run_browser`'s draw closure — renders the list + 1-row status bar into any given area
- [ ] In the draw closure: if `browser_state` is set, render via `render_browser` — into the art area if height ≥ 8, into the full terminal area otherwise
- [ ] In the event loop: if `browser_state` is set, route key events to browser navigation instead of player actions; on select/return/quit update state accordingly
- [ ] Remove `run_browser` from `src/browser/mod.rs`
