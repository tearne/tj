# Cue as Beat Grid Anchor
**Type**: Proposal
**Status**: Implementing

## Intent

The cue point and the beat grid are currently independent. Setting a cue records a position but does not move the ticks, so a tick rarely falls on the cue. The cue point is a musically meaningful position — a downbeat, a phrase start — and the beat grid should reflect that. The cue should act as the zero datum for tick marks: ticks snap to it when it is set, and BPM changes keep them anchored there.

## Specification Deltas

### ADDED

- When a cue point is set, the beat grid is immediately snapped so that a tick falls on the cue position (`offset_ms` is recalculated via the existing anchor logic).

### MODIFIED

- BPM tap while a cue is set: previously prompted for confirmation then cleared the cue. Now the cue is preserved and tapping proceeds normally — the tapped grid lands where it lands, with no re-anchor.
- Default key for Deck 1 cue play (previously Shift+A) is now Space+A. Default key for Deck 1 cue set (previously Space+A) is now Shift+A. Same swap for Deck 2: Space+D ↔ Shift+D.
- Cue play (Space+A/D): jumps to the cue point and maintains the current play state (playing stays playing, paused stays paused). Does nothing if no cue is set. Previously always started playback.
- Cue set (Shift+A/D): only acts when paused — sets the cue at the current position. Does nothing while playing. Previously also jumped to the existing cue and paused when playing.

### REMOVED

- The BPM tap confirmation prompt ("BPM tap will clear the cue point — tap again to confirm").
- `cue_tap_pending` field on `Deck` and its expiry logic in `service_deck_frame`.

## Scope

- **In scope**: cue-set grid snap; removal of tap confirmation; key swap for cue play vs cue set.
- **Out of scope**: offset changes (already free, already preserve the cue); BPM ± adjustments (already re-anchor, no change needed); BPM tap does not re-anchor.
