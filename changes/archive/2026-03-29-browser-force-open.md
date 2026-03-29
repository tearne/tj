# Browser Force Open

## Intent

When a track is playing, pressing the browser key shows an error and refuses to open. If the user presses again, they clearly want the browser open regardless — the error should yield to a second request rather than requiring the user to pause first.

## Approach

Mirrors the quit-confirmation pattern: pressing the browser key while a track is playing shows an error notification and sets a `browser_blocked: Option<(Instant, usize)>` flag (5-second expiry + target deck slot). The notification reads `"Track is playing — open browser?  [y] open   [Esc/n] cancel"`. While that flag is active, pressing `y` clears the notification, clears the flag, and opens the browser. `Esc` or `n` cancels. The flag expires alongside the notification.

## Plan

- [x] ADD `browser_blocked: Option<(Instant, usize)>` in `tui_loop`; set alongside the "can't browse" notification; clear on expiry, on `Esc`/`n`, and on any superseding notification
- [x] UPDATE global key handling: while `browser_blocked` is set, `y` opens the browser for the stored deck slot; `Esc`/`n` cancels
- [x] UPDATE SPEC/browser.md: note `y` override and `Esc`/`n` cancel
- [x] UPDATE version bump

## Conclusion

`browser_blocked: Option<(Instant, usize)>` added to `tui_loop`. Pressing the browser key while the target deck is playing sets the flag and shows `"Track is playing — open browser?  [y] open   [Esc/n] cancel"` with a 5-second countdown. `y` opens the browser immediately; `Esc`/`n` cancels. The flag and notification expire together after 5 seconds. SPEC/browser.md updated.
