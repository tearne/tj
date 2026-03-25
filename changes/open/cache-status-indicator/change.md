# Cache Status Indicator
**Type**: Proposal
**Status**: Draft

## Intent

When a track is loaded there is no visual summary of how much preparation has been done on it — whether BPM has been analysed, the beat grid tuned, or a cue point set. Three small indicators in the notification bar provide this at a glance, distinguishing "not set" (dark) from "established" (lit).

## Specification Deltas

### ADDED

**Cache status indicators** — When a track is loaded, three fixed-width single-character indicators are shown right-aligned in the notification bar, separated from the track name by a spacer.

| Indicator | Field | Lit | Dark |
|-----------|-------|-----|------|
| `[BPM]` | BPM | BPM has been established (analysis complete, or loaded from cache) | Analysis not yet complete |
| `[Tick]` | Offset | `offset_ms` is non-zero (beat grid has been manually tuned) | Offset is at default (0 ms) |
| `[Cue]` | Cue | A cue point is set for this track | No cue set |

"Lit" uses the active palette's treble colour (consistent with the track name style). "Dark" uses a near-black colour so the brackets and text are present but visually inactive.

The three indicators are displayed in a fixed-width group at the right edge of the notification bar at all times when a track is loaded, so the track name does not shift as state changes. They are absent when no track is loaded.

In vinyl mode all three indicators are always shown in the dark/inactive colour regardless of state — the underlying values are preserved but the BPM machinery is dormant. They return to normal lit/dark behaviour when switching back to beat mode.

### MODIFIED

**`SPEC/render.md` — Notification Bar** — Updated to document the indicators and their layout.

## Scope

- **In scope**: the three indicators; reading `analysis_hash`, `offset_ms`, and `cue_sample` from the deck at draw time.
- **Out of scope**: red/dirty state (changed but not yet saved); indicators on the global status bar.
