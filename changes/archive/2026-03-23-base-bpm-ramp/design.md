# Base BPM Ramp — Design

## Overview

`BaseBpmIncrease` and `BaseBpmDecrease` currently fire on `Press` only, stepping
±0.01 BPM per event. Handling both `Press` and `Repeat` (OS key-repeat) gives
continuous adjustment when held. A time-based step ramp makes large corrections
fast while keeping single-tap precision.

## State

Two variables in the outer event loop:

```rust
let mut bpm_ramp_started: Option<Instant> = None;
let mut bpm_ramp_last:    Option<Instant> = None;
```

`bpm_ramp_started` records when the current ramp began. `bpm_ramp_last` records
when the last base-BPM key event fired, used to detect a fresh tap.

## Event handling

On any `Press | Repeat` of a `BaseBpm*` action:

1. Compute `gap = bpm_ramp_last.elapsed()` (MAX if None).
2. If `gap > 80 ms` — reset `bpm_ramp_started = Some(Instant::now())`.
   This treats the event as a fresh tap; a quick release-and-repress
   (within 80 ms) continues the current ramp tier instead.
3. Set `bpm_ramp_last = Some(Instant::now())`.
4. Compute `elapsed` from `bpm_ramp_started`; pick step from the table below;
   apply ±step to `base_bpm`, clamp to 40–240 BPM, propagate speed.

```
elapsed < 3 s  →  0.01 BPM / event  (fine)
elapsed ≥ 3 s  →  0.05 BPM / event  (medium)
```

Step size is time-based rather than repeat-count-based so behaviour is
independent of the OS key-repeat rate. Because some terminals send key-repeat
as `Press` events rather than `Repeat`, the ramp does not rely on
`KeyEventKind::Repeat` — it treats any closely-spaced event as a continuation.

## No Release arm

No `Release` handling is needed. The gap check on the next `Press` determines
whether to continue or restart the ramp.

## No new actions or keymap changes

The ramp is purely a change to event-kind handling and step computation in the
four existing action arms. No new `Action` variants, no keymap changes.
