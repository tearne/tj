# Browser in Art Area
**Type**: Proposal
**Status**: Implementing

## Intent

When the browser is open and the art area has enough height (≥ 8 rows), render the browser there instead of as a fullscreen overlay. This keeps both decks visible while browsing, making track selection less disruptive.

## Approach

The browser is currently a fullscreen overlay. The proposed change:

- When the art area is tall enough (≥ 8 rows) and the browser is open, render the browser there instead of fullscreen.
- When the art area is too short, fall back to the existing fullscreen overlay — no regression for small terminals.
- Cover art is hidden while the browser occupies the art area.

## Scope

- Render routing: check art area height and direct browser to art area or fullscreen accordingly
- Browser render functions may need to accept an explicit area rather than assuming fullscreen
- No changes to browser logic, keymap, or state
