# Design: Overview Ticks Always Visible
**Status: Draft**

## Approach

Remove the `&& byte == 0` guard from the bar marker colour check in the overview rendering loop. Markers will then be drawn at their column regardless of waveform content.

## Tasks
1. ✓ Impl: Remove `&& byte == 0` guard from overview bar marker check
2. ✓ Verify: bar markers visible across dense waveform sections
3. ✓ Process: confirm ready to archive
