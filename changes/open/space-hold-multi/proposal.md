# Proposal: Space Held for Multiple Key Presses
**Status: Draft**

## Problem

Pressing `Space+j` then `Space+k` (e.g. to max both deck levels) requires releasing and re-pressing Space between each chord. Physically holding Space down and pressing `j` then `k` does not work — the second chord never fires.

## Root Cause

After each chord fires, `space_held` is reset to `false`:

```rust
// in the general action resolver
space_held = false;
Some(a)
```

The same explicit reset appears in the cue handlers. The comment in the SPEC explains why: *"ensuring regular key bindings work correctly on terminals that do not send key-release events."* Without a release event, `space_held` would remain `true` indefinitely after a chord, blocking all bare key presses.

## Why the Fix is Safe

The app already receives `KeyEventKind::Release` events for Space — Kitty's keyboard protocol sends them. The Release handler already clears `space_held`:

```rust
KeyEventKind::Release => { space_held = false; }
```

This means the post-chord reset is redundant when running under Kitty. Removing it will allow a physically held Space to remain active across multiple chord presses, with the Release event correctly clearing the flag when Space is genuinely released.

## Proposed Change

Remove the `space_held = false` resets that follow chord resolution:

1. In the general action resolver (after `SpaceChord` lookup).
2. In the `Deck1Cue` handler.
3. In the `Deck2Cue` handler.

The `Space` Release handler remains unchanged and continues to clear `space_held`.

## Risk

**Terminals without release events**: On a terminal that does not send `KeyEventKind::Release` for Space, removing the post-chord reset means `space_held` would never clear, blocking all bare key presses after the first chord. This is a regression for such terminals.

**Mitigation options** (to evaluate during implementation):
- Accept the regression — Kitty is the supported terminal, and release events are a hard requirement for warp-nudge already.
- Track whether any `Release` event has ever been received; if not, restore the post-chord reset as a fallback.
- Keep the reset but only apply it when an explicit release has not arrived within a short window (e.g. 50 ms) — effectively a hybrid.

## SPEC Update

Update `SPEC/keymap.md`:

> *Before*: The Space-held state resets when a chord action fires, ensuring regular key bindings work correctly on terminals that do not send key-release events.

> *After*: The Space-held state is cleared by the Space key-release event. Holding Space and pressing multiple keys fires each chord in sequence.

## Out of Scope

- Supporting terminals that do not send key-release events (a pre-existing constraint shared with warp-nudge).
