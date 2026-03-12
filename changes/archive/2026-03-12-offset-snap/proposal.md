# Proposal: Offset Step Alignment
**Status: Ready for Review**

## Intent
The beat phase offset (`offset_ms`) is adjusted in 10 ms steps, but is loaded from the cache as an arbitrary value. If the cached value is not a multiple of 10 ms (e.g. 7 ms), stepping with `+`/`-` cycles through …−3 ms, 7 ms, 17 ms… and 0 ms is unreachable. The same issue applies to `audio_latency_ms` if the cache was written before the calibration-entry snap was introduced.

## Specification Deltas

### MODIFIED
- `offset_ms` is snapped to the nearest 10 ms boundary when loaded from the cache, ensuring `+`/`-` steps always land on multiples of 10 ms and 0 ms is always reachable.
- `audio_latency_ms` is snapped to the nearest 10 ms boundary on load (not only on calibration entry), for the same reason.
