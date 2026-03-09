# Proposal: Remove Render Modes
**Status: Draft**

## Intent
The detail waveform currently supports two render modes (buffer and live), toggled at runtime with `m`. Buffer mode is the more capable approach and is now the established standard. Live mode adds complexity without a compelling use case.

## Specification Deltas

### REMOVED
- **Detail Waveform Render Modes**: The `m` key and the concept of switchable render modes are removed. Buffer mode behaviour becomes the sole rendering approach and is no longer described as a mode.

### MODIFIED
- **Detail Waveform**: The background thread pre-renders a buffer wider than the visible area; the UI thread slides a viewport through it. This is now the only rendering approach and requires no runtime configuration.
- **Key hints**: `m` is removed from the displayed key bindings.
