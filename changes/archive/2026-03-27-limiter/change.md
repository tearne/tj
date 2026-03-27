# Limiter
**Type**: Spike → Fix
**Status**: Approved

## Goal
Explore adding a per-deck limiter applied after the gain stage, so gain can be raised without causing clipping on loud passages.

## Questions
- Where in the audio source chain should the limiter sit — inside `FilterSource`, as a new wrapper, or elsewhere?
- Hard clip vs soft-knee: is soft-knee worth the complexity for a DJ context?
- Should the limiter ceiling be fixed (0 dBFS) or configurable?
- Any lookahead requirement, or is sample-by-sample sufficient?
- Does the limiter state need to be exposed in the UI (e.g. gain reduction indicator)?

## Log
