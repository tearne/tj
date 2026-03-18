# Layout

## UI Structure

The UI is structured into the following vertical sections (top to bottom):
1. Detail info bar (shared)
2. Detail waveform — Deck A
3. Detail waveform — Deck B
4. Notification bar — Deck A
5. Info bar — Deck A
6. Overview — Deck A
7. Notification bar — Deck B
8. Info bar — Deck B
9. Overview — Deck B
10. Global status bar

## Global Status Bar

A single row pinned to the bottom of the UI. Content priority:
1. **System notification** — transient messages not tied to either deck (e.g. config parse warning, startup prompt). Shown until expired; uses the same `Notification` type and expiry mechanism as per-deck notifications.
2. **Idle status** — shown when no system notification is active. Displays the current browser working directory in dim style.

## Notification Bar

- A single line displayed above the info bar. By default it shows the track name derived from embedded metadata: `Artist – Title` if both are present, `Title` if only a title is available, or the filename as a fallback. Shown only when a track is loaded.
- When a notification is active it temporarily replaces the track name. Notifications carry a message, a style (`Info` / `Warning` / `Error`), and an expiry; the most recent notification takes precedence. Notifications expire automatically after their timeout; no explicit dismissal is required.
- The track name is rendered in a muted form of the active palette's treble colour, distinguishing it visually from notification text.
- The BPM confirmation prompt (see [transport.md](transport.md)) is displayed as a `Warning`-style notification.
- If no config file is found on first launch, an `Info` notification briefly displays the path at which the default config was created, then the bar reverts to the track name.

## Info Bar

- A single line below the track name bar. Content is split into two groups separated by a variable-width spacer that fills remaining width, keeping the right group pinned to the right edge regardless of transient field changes:
  - **Left group**: play/pause icon (`▶`/`⏸`), BPM, `♪` in red when metronome is active, phase offset. Tap count (`tap:N`) appended transiently while a tap session is active.
  - **Right group**: nudge mode (`nudge:jump` / `nudge:warp`, fixed width), level (`level:▕N▏` — single eighth-block character in dark yellow, in a bracketed indicator with mid-grey brackets), `lat:Xms` (shown only when `audio_latency_ms > 0`), spectrum strip.
- The nudge mode field is always present and fixed-width so toggling between `jump` and `warp` does not shift anything to its right.
- When no tempo adjustment is active, the detected BPM is shown to two decimal places (e.g. `120.00`) and receives a soft amber beat-flash. When a `f`/`v` adjustment is active, the detected BPM is shown plain and the adjusted tempo is shown alongside in parentheses (e.g. `120.00 (124.40)`), with only the adjusted number receiving the beat-flash.
- Pressing `?` opens a modal key binding reference overlay; any key dismisses it.
- During BPM analysis the BPM field shows an animated spinner. When a confirmation is pending, the prompt appears in the notification bar (see [transport.md](transport.md)); the right group is always rendered normally.
- A BPM is considered "established" once it has been loaded from cache, set by tap, or adjusted with `f`/`v`/`F`/`V`. Only established BPM triggers confirmation on new detection.

## Empty Deck Panels

When no track is loaded in a deck slot, all deck sections render at full height with placeholder content:
- **Notification bar**: dim deck label ("A" or "B") and prompt "no track — press z to open the file browser".
- **Info bar**: `⏸  ---  +0ms` in dim style; level and filter widgets omitted.
- **Overview**: a faint flat horizontal line at the vertical midpoint, rendered via the braille pipeline with zero-amplitude peaks and 120 BPM tick marks.
- **Detail waveform**: a faint vertical line at the playhead column spanning the full height; all other columns blank.

Layout constraints are based on the loaded deck's `detail_height` (defaulting to 8 rows), so no section collapses to zero when a deck slot is empty.
