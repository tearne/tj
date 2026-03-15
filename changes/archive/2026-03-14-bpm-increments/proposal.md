# Proposal: BPM Increment Steps
**Status: Approved**

## Problem

The `f`/`v` keys adjust playback BPM by ±0.1. Because the underlying value is stored as f32, the displayed BPM accumulates floating-point error rather than stepping cleanly (e.g. 120.0 → 120.09999… → 120.19998…). The speed is also applied as a ratio (`bpm / base_bpm`), so the feel is percentage-based rather than absolute.

The `F`/`V` keys adjust the base BPM by ±0.01, which is too fine to use reliably.

## Proposed change

- `f`/`v` (BpmIncrease / BpmDecrease): change step from **0.1 → 1.0 BPM**
- `F`/`V` (BaseBpmIncrease / BaseBpmDecrease): change step from **0.01 → 0.1 BPM**

Both steps are now exact in f32 (multiples of powers of two are not, but 0.1 and 1.0 round-trip cleanly enough for display), and the increments feel intentional rather than sub-perceptual.
