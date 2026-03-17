# Design: UI Polish
**Status: Complete**

## 1 — Global notification bar moved to top

Both layout render paths (empty-deck handler and main handler) updated. In every layout variant the global bar is now `c[0]` and returned as the last tuple element, with all other constraints shifted down by one index.

## 2 — Uniform notification bar label colour and background

Removed `active_label_style` / `dim_label_style` distinction in both render paths:
- Single `label_style = Style::default().fg(Color::Rgb(40, 60, 100))`
- Single `notif_bg = Style::default().bg(Color::Rgb(20, 20, 38))`

Both are applied uniformly to all notification bar rows regardless of which deck is active. The palette-derived `(tr, tg, tb)` colour and its associated `palette_idx` lookup are removed from both paths.

## 3 — Slower analysis spinner

`SPINNER[frame_count % SPINNER.len()]` → `SPINNER[(frame_count / 6) % SPINNER.len()]`

At ~60 fps: ~1 full rotation per second.
