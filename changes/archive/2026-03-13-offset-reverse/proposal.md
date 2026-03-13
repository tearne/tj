# Proposal: Reverse Tick Offset Direction
**Status: Draft**

## Intent

The current default mapping has `+` (shift+=) increase `offset_ms` and `_` (shift+-) decrease it. This feels backwards: pressing `-` (and its shift equivalent `_`) should move the offset in the negative direction, and `=`/`+` in the positive direction. Swap the defaults.

## Specification Deltas

### MODIFIED
- Default binding for `offset_increase` changes from `"+"` to `"_"`.
- Default binding for `offset_decrease` changes from `"_"` to `"+"`.
