# Proposal: Tap-Guided BPM Re-detection
**Status: Ready for Review**

## Intent
After a tap session produces a BPM estimate, automatically re-run the audio analyser with the tapped value as a hint. The analyser can achieve sub-integer precision that tapping cannot, while tapping resolves the common octave-error problem (e.g. detector returns 60 when the true BPM is 120). The two techniques are complementary: tap for coarse correction, analyser for fine precision.

## Specification Deltas

### MODIFIED
- After a tap session reaches 8 taps and sets `base_bpm`, a background re-detection pass is triggered automatically using `fusion` mode, with the search window narrowed to ±5% of the tapped BPM (`min_bpm`/`max_bpm` and the legacy preferred range clamped around the tapped value).
- The analyser runs only on the audio segment spanning the tap session — from the first tap to the last, with one beat of padding on each side. This is faster than analysing the full track and avoids other sections of the track interfering with the result.
- While re-detection is running the animated indicator appears as normal.
- If re-detection returns a result, `base_bpm` is updated to the analyser's value (which may differ fractionally from the tapped value). The tap-derived `offset_ms` is preserved — the analyser does not determine phase.
- If the tap session resets before re-detection completes, the in-flight result is discarded.

## Scope
- **In scope**: narrowing the BPM search window using the tap result; restricting analysis to the tapped segment; using fusion mode for best accuracy.
- **Out of scope**: using tap timing to guide beat phase detection in the analyser; exposing the search window as a user-configurable parameter.
