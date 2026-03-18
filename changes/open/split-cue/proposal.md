# Proposal: Split Cue Mode
**Status: Draft**

## Overview

Split cue mode turns the audio output into a stereo monitor for both decks simultaneously:

- Deck 1 is routed to the **left channel only**.
- Deck 2 is routed to the **right channel only**.
- Level and filter controls are **bypassed** — both decks play at full, unfiltered volume regardless of their mixer settings.

This lets the DJ monitor both decks independently in headphones without a second output device, before a dedicated house/master output is implemented.

## Key

| Key | Action |
|-----|--------|
| `\` | Toggle split cue mode on / off |

## Behaviour

### Normal mode (off)
Audio is mixed as usual. Both decks contribute to both channels, respecting level and filter settings.

### Split cue mode (on)
- Deck 1 audio is panned fully left (right channel zeroed).
- Deck 2 audio is panned fully right (left channel zeroed).
- Level multiplier is fixed at 1.0 for both decks, regardless of the `level` field.
- Filter is bypassed for both decks, regardless of `filter_offset`.
- The level and filter controls remain functional — their values are preserved and take effect again when split cue is turned off.

### UI indication
A `[split cue]` label (or similar) is shown in the global status bar while active, styled in a distinct colour (e.g. amber) so it is always visible.

## Future: House Output
A future change will add a second audio output (house/master). When that exists:
- The **current output** becomes the cue/monitor output — split cue mode applies here.
- The **house output** plays the normal mixed signal, respecting all level and filter settings, unaffected by split cue mode.

Split cue mode therefore has no effect on the house output.

## Implementation Notes

- Add a `SplitCueSource` wrapper (or equivalent channel-masking step) in the audio source chain, between the existing `FilterSource` and the mixer. In split cue mode it zeros the opposite channel for each deck; in normal mode it passes through unchanged.
- Level bypass: in split cue mode, the `FilterSource` level multiplier is overridden to 1.0 at the point it is applied to the output buffer (not stored — the `level` field is unchanged).
- Filter bypass: `filter_offset` is treated as 0 in split cue mode (filter coefficients are not recalculated; the IIR state is maintained so there is no click on deactivation).
- Split cue state is global (not per-deck) and is not persisted between sessions.

## Out of Scope

- Per-deck cue routing (cue bus / PFL).
- The house/master output itself (deferred to a future change).
