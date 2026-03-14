# Design: Notification Bar
**Status: Approved**

## Approach

The track name bar gains a priority-ordered render: BPM confirmation prompt (if pending) → active notification (if not expired) → track name (default).

### Notification state

Add to `tui_loop` state:

```rust
enum NotificationStyle { Info, Warning, Error }
struct Notification {
    message: String,
    style:   NotificationStyle,
    expires: Instant,
}
let mut active_notification: Option<Notification> = None;
```

Rendered colours: `Info` → `DarkGray`, `Warning` → `Yellow`, `Error` → `Red`.

A helper inline sets `active_notification` with an expiry:

```rust
active_notification = Some(Notification {
    message: "config created: ~/.config/tj/config.toml".into(),
    style:   NotificationStyle::Info,
    expires: Instant::now() + Duration::from_secs(5),
});
```

Expired notifications are cleared at the top of each render frame.

### BPM confirmation — moves to notification bar

`pending_bpm` stays as existing state. The render logic for it moves from the info bar right group into the notification bar:

- Full width: `BPM detected: 124.40  [y] accept  [n] reject  (14s)`
- Rendered as `Warning` (yellow) — it is a question, not an error
- Countdown number highlighted red when ≤ 5 s remaining
- Info bar right group is always rendered normally (no more replacement)

### Config notice — formalised

Remove the ad-hoc `config_notice` / `config_notice_start` pair. On entry to `tui_loop`, if `config_notice` is `Some(msg)`, post it as a `Notification` with 5 s expiry and `NotificationStyle::Info`.

### Track load errors

Fatal decode/audio errors currently exit the process after `cleanup_terminal()`. These are presented via `color-eyre`'s colourised output (already in place) and are not suitable for TUI notification — the TUI is not running at that point. No change needed here.

### Render priority (notification bar)

```
if pending_bpm.is_some()       → BPM confirmation prompt (Warning/yellow, countdown red when low)
else if active_notification
     && !expired               → notification message (Info/Warning/Error colour)
else                           → track name (DarkGray)
```

## Tasks

1. **Impl**: Add `NotificationStyle` enum and `Notification` struct; add `active_notification` state; post config notice through it; remove `config_notice` / `config_notice_start`
2. **Impl**: Move BPM confirmation render from info bar right group to notification bar
3. **Process**: Build clean, ready to archive
