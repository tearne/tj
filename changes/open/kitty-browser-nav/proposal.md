# Proposal: Kitty Browser Navigation Bug
**Status: Draft**

## Problem

When using the file browser in Kitty terminal, pressing Enter to navigate into a directory immediately navigates one level deeper into the first subdirectory, without any additional keypress from the user. This does not occur in WezTerm or Alacritty.

## Likely Cause

Kitty's keyboard protocol sends additional key events (e.g. key-repeat or release events) that arrive in the event queue slightly after the navigation completes. By the time the browser loop processes the next event, a spurious Enter is waiting in the queue, and since the cursor has landed on the first content entry of the new directory, it immediately navigates deeper.

A 200ms Enter cooldown after navigation was tried and confirmed to fix the issue, but was rejected as a bodge. The root cause should be identified and fixed properly.

## Investigation

- Determine exactly what events Kitty is sending around an Enter keypress (press, repeat, release) using crossterm's event stream
- Check whether crossterm's Kitty keyboard protocol support is correctly filtering repeat/release events before they reach the browser
- Consider whether the fix belongs in the event handling (filter spurious events) or the UX (cursor positioning that makes a spurious Enter harmless)

## Out of Scope

- Other terminals (WezTerm, Alacritty unaffected)
