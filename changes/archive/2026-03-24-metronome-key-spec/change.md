# Metronome Key Spec
**Type**: Fix
**Status**: Approved

## Log

The active-deck controls table in `SPEC/config.md` had three entries that were actually per-deck fixed bindings with wrong descriptions. The per-deck fixed table was missing four rows entirely. Corrections:

- `b` moved from active-deck to per-deck fixed as "Deck A tap BPM"; `n` added as "Deck B tap BPM"
- `'` description corrected from "Toggle metronome" to "Deck A BPM re-detect"; `#` added as "Deck B BPM re-detect"
- `@` description corrected from "Trigger manual BPM re-detection" to "Deck A tempo reset"; `~` added as "Deck B tempo reset"
- `B` / `N` added to per-deck fixed as "Deck A/B metronome toggle"
