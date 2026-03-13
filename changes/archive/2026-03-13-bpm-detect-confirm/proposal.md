# Proposal: BPM Detection Confirmation Step
**Status: Approved**

## Intent

When auto-detection completes and a BPM is already established, the result should be held as pending and require explicit user confirmation before being applied. This prevents jarring jumps when detection is wrong and the user has already set the correct BPM.

## Specification Deltas

### MODIFIED

- When auto-detection completes and no BPM is yet established (fresh load, nothing in cache, no tap or f/v adjustment made), the result is applied immediately as before.
- When auto-detection completes and a BPM is already established (loaded from cache, or user has tapped or adjusted with `f`/`v`), the result is held as pending. The info bar right group is replaced with a red confirmation prompt showing the detected BPM and a countdown (e.g. `BPM detected: 124.40  [y] accept  [n] reject  (15s)`).
- Pressing `y` (or `Enter`) accepts: applies the detected BPM and offset, persists to cache.
- Pressing `n` (or `Esc`) rejects: discards the detected result, leaving the pre-existing BPM and offset unchanged.
- After 15 seconds with no response, the pending result is auto-rejected and discarded. The countdown is shown in the prompt.
- Tap-detection is unaffected — it applies immediately as a deliberate manual act.
- `@` manually triggers a fresh re-detection pass at any time. The result always goes through the confirmation step regardless of whether a BPM is established.

## Notes

- The background thread currently sends cached and freshly-detected BPM through the same channel. The message must be tagged to indicate whether it is from fresh detection or cache, so the UI thread knows whether to trigger confirmation.
- "BPM already established" is tracked on the UI side: true if BPM was loaded from cache, or if the user has tapped or used `f`/`v` during the session.
