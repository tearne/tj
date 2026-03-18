# Proposal: Cue Points
\*\*Status: Implemented — v0.5.89\*\*

## Keys

| Key | Deck | Action |
|-----|------|--------|
| `Space+A` | 1 | Cue |
| `Space+D` | 2 | Cue |
| `A` | 1 | Jump to cue and play |
| `D` | 2 | Jump to cue and play |

## Behaviour

### While paused
Set the cue point at the current position.

### While playing — cue exists
Jump to cue and pause.

### While playing — no cue
No-op.

### Jump to cue and play (`A` / `D`)
Seek to the cue point and resume playback regardless of current state. No-op if no cue is set.

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

- `cue_sample` is stored as the raw `smooth_display_samp` (buffer fill position). Display position subtracts latency only while playing; when paused there is no buffer fill ahead so the raw position equals the heard position.
- Seek targets use `cue_sample` directly — no latency offset in the seek call.
- `CuePlay` pre-loads `smooth_display_samp` to `cue_sample + latency_samps` so the display is immediately correct once the buffer fills.
- BPM anchoring: on each BPM change, recalculate `offset_ms` so that `cue_sample / sample_rate` lands on a beat boundary at the new BPM.

## Visual Markers

Both the detail and overview waveforms show a solid green braille block (`⣿`) at the cue position. When the cue column coincides with the playhead column, the top and bottom rows show the cue colour so the cue line remains visible at both edges.

## Out of Scope

- Multiple cue points / hot cues
- Loop in/out markers
