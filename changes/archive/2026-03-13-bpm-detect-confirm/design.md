# Design: BPM Detection Confirmation Step
**Status: Approved**

## Approach

### Channel tagging

Change the `bpm_rx` channel payload from `(String, f32, i64)` to `(String, f32, i64, bool)` where the fourth field `is_fresh` is `true` for freshly detected BPM and `false` for cached. The UI thread uses this to decide whether to trigger confirmation.

### "BPM established" flag

Add `bpm_established: bool` on the UI side, initially `false`. Set to `true` when:
- A cached BPM is received via `bpm_rx` (`is_fresh = false`)
- The user taps (`b`) — set after the first 8-tap result is applied
- The user adjusts with `f`/`v`/`F`/`V`

### Pending confirmation state

Add `pending_bpm: Option<(String, f32, i64, Instant)>` — stores `(hash, bpm, offset_ms, received_at)` when a fresh detection result arrives and `bpm_established` is true.

When `pending_bpm` is `Some`:
- The info bar right group is replaced entirely with a red confirmation prompt:
  `BPM detected: 124.40  [y] accept  [n] reject  (Ns)`
  where `N` counts down from 15 to 0.
- `y` / `Enter` — apply `pending_bpm`, persist to cache, clear `pending_bpm`.
- `n` / `Esc` — discard `pending_bpm`, leave current BPM/offset unchanged.
- After 15 seconds, auto-reject: clear `pending_bpm`, leave current state unchanged.
- All other controls continue to function normally while confirmation is pending.

### Manual re-detection (`@`)

Add `Action::RedetectBpm`. On trigger: spawn a fresh detection thread (same as initial load but unconditional), set `analysing = true` spinner. Result always goes through the confirmation path regardless of `bpm_established`.

### `@` key binding

Add `redetect_bpm = "@"` to `config.toml` defaults and key binding table.

## Tasks

1. ✓ Impl: tag `bpm_rx` channel with `is_fresh: bool`; set `bpm_established` flag
2. ✓ Impl: pending confirmation state — store result, render red info bar prompt with countdown
3. ✓ Impl: `y`/`Enter` accept and `n`/`Esc` reject handlers
4. ✓ Impl: 15-second auto-reject
5. ✓ Impl: `@` manual re-detection action and key binding
6. ✓ Process: archive
