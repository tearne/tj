# Proposal: Filter Steps Increase
**Status: Ready for Review**

## Intent
Increase filter resolution from 10 to 20 steps per direction, keeping the same frequency range (40 Hz – 18 kHz), for finer tonal control.

## Specification Deltas

### MODIFIED
- Filter offset clamped to ±20 (was ±10).
- `FILTER_CUTOFFS_HZ` expanded to 20 log-spaced entries from 18 kHz down to 40 Hz (was 10 entries).
