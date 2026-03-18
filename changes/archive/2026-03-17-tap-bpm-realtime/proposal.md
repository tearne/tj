# Proposal: Tap BPM Realtime Ticks
**Status: Implemented — v0.5.81**

## Problem

Tapping on a track with no established BPM gives no visual feedback during the session:

- The BPM/ticks stay red (`unconfirmed` style) throughout.
- Beat tick marks don't appear in the detail waveform.
- Both resolve only at the **end** of the tapping session (after the 2-second timeout).

On a track with a pre-existing BPM this doesn't happen — after 8 taps, the ticks update in realtime on every subsequent tap.

## Fix

From the 8th tap onward, set `bpm_established = true` in the realtime tap path (currently only set at session end). Same one-line addition in both deck tap handlers.

## Effect

- From the 8th tap onward, tick marks appear and update in realtime on every tap.
- The BPM value renders in normal (confirmed) colour immediately.
- Behaviour on tracks with a pre-existing BPM is unchanged.
- The session-end path already sets `bpm_established = true` — becomes redundant for the `>= 8` case but harmless.

## Risk

Very low. One-line addition in two symmetric blocks. No data structure changes.
