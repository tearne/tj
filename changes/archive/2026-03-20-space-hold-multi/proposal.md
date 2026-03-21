# Proposal: Space Held for Multiple Key Presses
**Status: Archived**

## Problem

Pressing `Space+j` then `Space+k` (e.g. to max both deck levels) requires releasing and
re-pressing Space between each chord. Physically holding Space down and pressing `j`
then `k` does not work — the second chord never fires.

## Root Cause

After each chord fires, `space_held` is reset to `false`:

```rust
// in the general action resolver
space_held = false;
Some(a)
```

The same explicit reset appears in the cue handlers. The comment in the SPEC explains
why: *"ensuring regular key bindings work correctly on terminals that do not send
key-release events."* Without a release event, `space_held` would remain `true`
indefinitely after a chord, blocking all bare key presses.

## Why the Fix is Safe on Kitty

The app already receives `KeyEventKind::Release` events for Space — Kitty's keyboard
protocol sends them. The Release handler already clears `space_held`:

```rust
KeyEventKind::Release => { space_held = false; }
```

This means the post-chord reset is redundant under Kitty. Removing it will allow a
physically held Space to remain active across multiple chord presses, with the Release
event correctly clearing the flag when Space is genuinely released.

## Proposed Change

### 1. Remove post-chord resets

Remove the `space_held = false` resets that follow chord resolution:

1. In the general action resolver (after `SpaceChord` lookup).
2. In the `Deck1Cue` handler.
3. In the `Deck2Cue` handler.

The `Space` Release handler remains unchanged and continues to clear `space_held`.

### 2. Space-modifier indicator in the global info bar

When `space_held` is true, show a visible indicator in the global info bar (e.g.
`[SPC]`). This gives the user immediate feedback that the modifier is active, and
makes a stuck `space_held` state obvious — on any terminal.

### 3. Config option: `space_chord_auto_reset`

Add a boolean config parameter (default `false`) that restores the old behaviour:
when `true`, `space_held` resets to `false` after each chord fires, matching the
current behaviour. Users on terminals that do not send key-release events can set
this to recover the original behaviour.

```toml
[keys]
space_chord_auto_reset = true   # set if your terminal does not send key-release events
```

## Risk

**Terminals without release events**: without `space_chord_auto_reset = true`, `space_held`
would never clear after a chord, blocking all bare key presses. The indicator (§2) makes
this immediately visible; the config option (§3) provides the escape hatch.

## SPEC Updates

Update `SPEC/keymap.md`:

- The Space-held state is cleared by the Space key-release event. Holding Space and
  pressing multiple keys fires each chord in sequence.
- When `space_held` is true, the global info bar shows a `[SPC]` indicator.
- Config: `space_chord_auto_reset` (bool, default `false`). When `true`, `space_held`
  resets after each chord — use on terminals that do not send key-release events.

## Out of Scope

- Supporting terminals that do not send key-release events as a first-class target
  (a pre-existing constraint shared with warp-nudge).
