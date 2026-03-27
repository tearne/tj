# Vinyl Jump Times
**Type**: Fix
**Status**: Approved

## Log
Vinyl jump times were left at the old values (0.5s, 2s, 8s, 32s) after the key assignment changed from (1, 4, 16, 64 beats) to (16, 32, 1, 4 beats). The times are derived from N beats × 0.5s at 120 BPM, so the correct values are 8s, 16s, 0.5s, 2s respectively. SPEC table updated to match.
