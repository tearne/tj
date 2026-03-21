# Proposal: Space Hold Multi-Chord (v2)
**Status: Draft**

## Background

The original `space-hold-multi` proposal was implemented and archived (2026-03-20). It removed
the post-chord `space_held = false` resets so that physically holding Space across multiple
chord presses would work. A `[SPC]` indicator was added to the detail info bar to show when
the modifier is active.

The implementation was immediately broken in practice: `space_held` became permanently sticky
after the first chord press on Kitty.

## Root Cause

crossterm 0.29 does not correctly decode Kitty keyboard protocol events for ASCII keys:

- **Repeat events** are decoded as `KeyEventKind::Press` rather than `KeyEventKind::Repeat`.
- **Release events** do not arrive at all — diagnostic counters (P, Rep, R) showed P:42 R:0
  after holding Space for ~2 seconds; P climbed with every OS key-repeat cycle confirming
  repeats-as-Press, R never incremented confirming no Release events.

Adding `DISAMBIGUATE_ESCAPE_CODES` alongside `REPORT_EVENT_TYPES` made no difference.

After each chord fires, the post-chord `space_held = false` reset was immediately undone by
the continuing stream of OS key-repeat events (decoded as Press), re-arming `space_held`
before the next key was pressed.

## Resolution: Suppress-Until-Silence

After a chord fires, suppress further Space Press/Repeat events until one full frame passes
with no Space activity. That silence indicates physical release.

```
chord fires           →  space_held = false, space_repeat_suppressed = true
repeat events arrive  →  ignored (suppressed)
key physically released, repeats stop
next frame, no Space  →  space_repeat_suppressed = false
Space press works     →  space_held = true as normal
```

Implementation:
- `space_repeat_suppressed: bool` and `space_saw_event_this_frame: bool` added alongside `space_held`
- Frame loop start: if `space_repeat_suppressed && !space_saw_event_this_frame` → clear suppression; reset `space_saw_event_this_frame`
- Space handler: set `space_saw_event_this_frame = true` on any Space event; ignore Press/Repeat while suppressed; Release clears both flags immediately
- All three chord reset sites set `space_repeat_suppressed = true` alongside `space_held = false`
- `[SPC]` indicator restored in detail info bar

Multi-chord (holding Space across multiple chord presses) remains unimplemented — the
post-chord reset still fires. This fix only addresses the stickiness regression.
