# Global Notification Style

## Intent

Global notifications other than errors are currently rendered as plain dim text with no countdown and no way to dismiss them. This change gives all global notifications a consistent treatment — centred text, countdown timer, and Esc to dismiss — with a per-style colour scheme. The config-created notice specifically gets a green-on-blue theme.

## Approach

Generalise the existing `error_bar` closure in the render block into a `notification_bar` that accepts foreground and background colours. Apply it to all `NotificationStyle` variants, each with its own palette:

| Style   | Foreground              | Background             |
|---------|-------------------------|------------------------|
| Error   | `Rgb(255, 180, 180)`    | `Rgb(100, 20, 20)`     |
| Warning | `Rgb(255, 220, 120)`    | `Rgb(80, 60, 0)`       |
| Info    | `Rgb(160, 200, 255)`    | `Rgb(20, 40, 80)`      |
| Success | `Rgb(140, 230, 160)`    | `Rgb(10, 60, 30)`      |

Config-created notification changed from `NotificationStyle::Info` to `NotificationStyle::Success` so it renders green on blue.

Add an Esc handler: when a global notification is active and no overlay (browser, tag editor, quit confirm, `browser_blocked`) is blocking input, Esc clears it immediately. Interactive confirmations (`pending_quit`, `browser_blocked`) already intercept Esc before the generic handler — no conflict.

Standardise all notification expiry to a single constant `NOTIFICATION_TIMEOUT: Duration = 5s`. The "no track loaded" hint and beat-period durations are unrelated and unchanged. `Warning` style is defined speculatively with the yellow-on-dark palette; unused in the global bar for now.

## Plan

- [x] ADD `NOTIFICATION_TIMEOUT: Duration` constant (5s); replace all inline `from_secs` expiry literals on notifications with it — both global and deck-bar (covers 3s rename success, 3s tags saved, 8s config-created, 10s load failed)
- [x] REFACTOR `error_bar` → `notification_bar(fg, bg)` in the render block; pass colours per style
- [x] UPDATE config-created notification: `Info` → `Success`
- [x] ADD Esc dismiss for active global notification (no-op when other overlays are present)
- [x] UPDATE SPEC: document Esc-to-dismiss convention and per-style colours
- [x] UPDATE version bump

## Conclusion

`NOTIFICATION_TIMEOUT` (5s) defined in `deck/mod.rs` and applied to all notification expiry across global and deck-bar notifications. `error_bar` generalised to `notification_bar(fg, bg, countdown_fg)`; all four `NotificationStyle` variants now render with centred text, countdown, and per-style colour scheme. Config-created notification changed to `Success` (green on blue). Esc dismisses any active global notification, with `suppress_quit_until` guard to absorb Kitty duplicate events. SPEC/render.md updated with priority table and colour scheme.
