# Proposal: BPM Manual Entry
**Status: Draft**

## Intent
BPM auto-detection occasionally fails or produces an incorrect value. The `h`/`H` halve/double correction handles common multiplier errors but there is no way to fine-tune the detected BPM itself. This proposal adds `Alt+F` / `Alt+V` to adjust `base_bpm` (the native track tempo) by ±0.1, complementing the existing `f`/`v` keys which adjust playback speed relative to that base.

## Specification Deltas

### ADDED
- `Alt+F` increases `base_bpm` by 0.1; `Alt+V` decreases it by 0.1. Both clamp to 40.0–240.0.
- Adjusting `base_bpm` resets any active `f`/`v` playback offset (`bpm` is set equal to the new `base_bpm` and playback speed returns to 1×), so the beat grid and playback tempo stay in sync after correction.
- The new BPM is persisted to the cache immediately.
- `Alt+F` and `Alt+V` are added as configurable mappable functions (`base_bpm_increase`, `base_bpm_decrease`).

## Scope
- **Out of scope**: free-text numeric entry for BPM (may be reconsidered separately if fine-stepping proves insufficient).
