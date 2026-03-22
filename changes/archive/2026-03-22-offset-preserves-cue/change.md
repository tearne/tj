# Change: Offset preserves cue point
**Type**: Fix

## Problem

Changing the tick offset while a cue point is set triggers a confirmation warning and, on second press, clears the cue. But the cue marks a musical position independent of the beat grid — if the cue is correct but the ticks are wrong, clearing the cue is a step backwards.

## Fix

Remove the offset-change confirmation and cue-clearing logic entirely. Offset changes now apply immediately regardless of cue state. Remove the `cue_offset_pending` field from `Deck`.

## Log

- Removed cue-clearing blocks from all four offset action handlers.
- Removed `cue_offset_pending` expiry in `service_deck_frame`.
- Removed `cue_offset_pending` field from `Deck` struct and its initialisation.
