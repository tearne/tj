# Consistent Past-End Waveform

## Intent
The detail waveform shows a flat zero-amplitude line for columns past the end of a track, but blank space for columns before the start. These should be consistent: no wave line in either region.

## Approach
In `peaks_for_slot` (`src/render/mod.rs`), the past-end branch returns `(0.0, 0.0)`, which `render_braille` draws as a flat zero-amplitude line. Change it to return the same `(1.0, -1.0)` inverted sentinel used for the pre-start region — `render_braille` skips any column where `min > max`, leaving it blank.

## Plan
- [x] UPDATE IMPL — `peaks_for_slot`: change past-end return from `(0.0, 0.0)` to `(1.0, -1.0)`

## Conclusion
Single-value change in `peaks_for_slot` (`src/render/mod.rs`). Past-end columns now return the same `(1.0, -1.0)` inverted sentinel as pre-start columns, so both regions render as blank space.
