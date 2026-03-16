# Proposal: Cue Points
**Status: Approved**

## Keys

- `Space+A` — Deck 1 cue
- `Space+D` — Deck 2 cue

## Behaviour

Modelled on CDJ cue behaviour, adapted for keyboard (press/release events available via `REPORT_EVENT_TYPES`).

### While playing — tap cue
Pause and snap to the cue point. The position where playback stops becomes the new cue point.

### While paused — hold cue
Play from the cue point for as long as the key is held. On release, pause and snap back to the cue point. This is the CDJ "preview" behaviour.

### While paused — tap cue (no cue point set)
Set the cue point at the current position.

### Setting a new cue point manually
`Shift+cue key` sets the cue point at the current position regardless of playback state, without affecting playback.

- Deck 1: `Shift+A`
- Deck 2: `Shift+D`

## Interaction with BPM and Offset

### BPM tap
Tapping a new BPM while a cue point is set would make the beat grid inconsistent with the cue point. Before the first tap is registered, the deck's info bar shows a confirmation message:

> "BPM tap will clear the cue point — press tap again to confirm"

A second tap within a short window confirms and proceeds (clearing the cue point and tapping normally). Any other key dismisses the prompt without tapping. This follows the same pattern as the existing pending-BPM confirmation.

### Manual BPM adjustment (f/v / s/x keys)
When a cue point is set, BPM adjustments are **anchored to the cue point**: the tick offset is automatically recalculated after each BPM change so that the beat grid remains aligned to the cue position. This keeps the cue point musically correct as the user nudges tempo.

### Tick offset adjustment
Manually changing the tick offset while a cue point is set would shift the beat grid away from the cue. The info bar shows:

> "Offset change will clear the cue point — press offset again to confirm"

Same two-keypress confirmation pattern as BPM tap. On confirmation, the cue point is cleared and the offset change is applied.

## Persistence

The cue point (sample position) is saved to the JSON cache alongside BPM and offset, keyed by track hash. It is restored when the track is loaded.

## Implementation Notes

- Hold-to-play uses `KeyEventKind::Press` to begin and `KeyEventKind::Release` to end — the same pattern as nudge. `REPORT_EVENT_TYPES` is already active.
- A "cue active" flag is needed per deck to track whether the key is currently held, so the release event knows to return to cue rather than just stop.
- Snapping to cue uses the existing seek mechanism.
- BPM anchoring: on each BPM change, recalculate `offset_ms` so that `cue_sample / sample_rate` lands on a beat boundary at the new BPM.

## Visual Markers

Both the detail and overview waveforms show a vertical green line at the cue position, overlaid on the waveform similarly to existing tick marks. The marker is visible regardless of playback state or which deck is active.

## Out of Scope

- Multiple cue points / hot cues
- Loop in/out markers
