# Experiment: Filter Bypass on Play
**Status: Adopted**

## Question

When the transport resumes from paused — with or without the filter having been moved while paused — a brief burst of unfiltered (or less-filtered) audio is audible. Can this be reliably eliminated?

## Log

### Root cause analysis

The audio chain is: `FilterSource<TrackingSource>` → `Speed` → `TrackPosition` → `Pausable` → `Amplify` → `Skippable` → `Stoppable` → `PeriodicAccess` → mixer.

`Pausable` outputs zeros without consuming from the inner source when paused. `FilterSource` is therefore never called while paused, so its IIR biquad state (x1, x2, y1, y2 per channel) stagnates.

On resume:
- If the filter was moved while paused: `last_offset` in FilterSource still holds the pre-pause value; `recompute_coefficients` fires on the next call with the new offset and the stale IIR state → impulse burst.
- Even without moving the filter: the IIR state reflects the last samples from before the pause. If the filter is active (non-zero offset), resuming from silence into a non-zero-state biquad still produces a transient.

Both cases have the same fix: zero the IIR state on resume when the filter is active, and apply a short fade-in to mask the settling transient.

### Fix implemented (v0.5.104)

Added `filter_state_reset: Arc<AtomicBool>` shared between `DeckAudio` and `FilterSource`:
- `FilterSource::next()`: if the flag is set, zero all per-channel state (x1, x2, y1, y2), reset `last_offset` to force a fresh `recompute_coefficients` call, and clear the flag.
- `PlayPause` (paused → playing): if `filter_offset != 0`, set the flag and store `FADE_SAMPLES` into `fade_remaining` to fade in over ~5.8ms.
- `CuePlay`: same — set flag and fade-in after `seek_direct`.

The fade-in ensures that even the brief biquad settling from zero-state is inaudible, since the input samples ramp up from silence.

## Outcome

Fix confirmed working in v0.5.105.

First attempt (v0.5.104) faded the **input** to the IIR (via TrackingSource fade_remaining). This was insufficient: the IIR warmed up on near-zero samples, which doesn't produce correct steady-state history — particularly for HPF where b0 ≈ 1, so the first real output samples after the fade were still inconsistent with the filter.

Second attempt (v0.5.105) fades the **output** of FilterSource instead. The IIR processes real audio from the first sample (correct convergence), while any settling transient is masked by a ~5.8ms ascending ramp on the output. IIR state is still zeroed on reset for a clean starting point.

No spec change required — this is a bug fix to existing filter behaviour.

