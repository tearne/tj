# Tap Paused Guard
**Type**: Fix
**Status**: Done

## Problem

Tapping BPM on a paused deck records tap times and shows the `tap:N` text in the
info line. Tapping while paused is meaningless — there is no playback position
advancing to align a beat grid against.

## Fix

Add an `is_paused()` early-return guard to the `Deck1BpmTap` and `Deck2BpmTap`
action handlers (lines 1371 and 1393). If the deck is paused, do nothing.

Because `tap_times` is never populated when paused, the `tap_active` check in
`info_line_for_deck` and the tap timeout logic in the service loop both naturally
stay silent — no further changes needed.

## Log

Wrapped the body of both `Deck1BpmTap` and `Deck2BpmTap` handlers in
`if !d.audio.player.is_paused()`. No other changes needed.
