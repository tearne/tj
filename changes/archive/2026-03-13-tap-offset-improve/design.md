# Design: Improve Tap-Informed Offset
**Status: Approved** *(retrospective)*

## Approach

Investigation proceeded iteratively:

1. Re-derived offset using re-detected BPM (rather than preserving stale tap-derived offset) — marginal improvement.
2. Disabled re-detection entirely — confirmed as primary instability source.
3. Replaced median inter-tap BPM with linear regression — BPM now converges as taps accumulate.
4. Added two-pass outlier filter — drops taps with residual > half a beat period before final regression.
5. Removed re-detection path and all associated dead code permanently.

## Tasks

1. ✓ Impl: re-derive offset using re-detected BPM on tap-guided result
2. ✓ Impl: disable re-detection (diagnostic test — confirmed as root cause)
3. ✓ Impl: replace median BPM with linear regression
4. ✓ Impl: add outlier tap filter (two-pass regression)
5. ✓ Impl: remove re-detection path and dead code
6. ✓ Process: archive
