# Proposal: Inactive Deck Spectrum Analyser
**Status: Draft**

## Intent

The spectrum analyser strip in the inactive deck's info bar is frozen — it never updates while the deck is inactive. This reads as a severely degraded framerate compared to the active deck.

## Specification Deltas

### MODIFIED

- **Spectrum analyser**: previously updated only for the active deck; now updated for both decks at the same half-beat / 8-beat cadence regardless of which deck is active.
