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
| `B` | BPM | BPM has been established (analysis complete, or loaded from cache) | Analysis not yet complete |
| `O` | Offset | `offset_ms` is non-zero (beat grid has been manually tuned) | Offset is at default (0 ms) |
| `C` | Cue | A cue point is set for this track | No cue set |

"Lit" uses the active palette's treble colour (consistent with the track name style). "Dark" uses a near-black colour so the characters are present but visually inactive.

The indicators occupy fixed width at all times so the track name does not shift as state changes. They are absent when no track is loaded.

### MODIFIED

**`SPEC/render.md` — Notification Bar** — Updated to document the indicators and their layout.

## Scope

- **In scope**: the three indicators; reading `analysis_hash`, `offset_ms`, and `cue_sample` from the deck at draw time.
- **Out of scope**: red/dirty state (changed but not yet saved); indicators on the global status bar.
