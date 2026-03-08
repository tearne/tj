# Proposal: BPM Correction
**Status: Ready for Review**

## Intent
Allow the user to correct an incorrect BPM detection at runtime, without reloading the track. Covers the two most common failure modes: metrical level errors (half/double-time) and algorithm disagreement.

## Specification Deltas

### ADDED

**BPM correction controls**:
- `h` halves the current BPM; `H` doubles it. Takes effect immediately — beat markers, flash indicator, and beat jump distances all update on the next frame. The corrected value is persisted to the cache on quit.
- `r` re-runs BPM detection on the in-memory audio using an alternative algorithm configuration. Audio continues playing during detection. The BPM display and all derived visuals update when detection completes. The new value is written to the cache immediately.
- Each press of `r` cycles through detection modes in order:
  1. Default (tempogram, initial detection — shown as `[auto]`)
  2. Fusion — runs tempogram and legacy algorithm in parallel, picks the best (`[fusion]`)
  3. Legacy — forces the autocorrelation + comb filter algorithm (`[legacy]`)
  4. Cycles back to default
- The current detection mode is shown alongside the BPM in the UI.

### MODIFIED
- **Player Controls** table: add `h`/`H` → "Halve / double BPM" and `r` → "Re-detect BPM".
- **Beat Detection** behaviour: note that the user can correct BPM at runtime and that corrections are persisted to the cache.
