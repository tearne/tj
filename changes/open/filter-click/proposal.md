# Proposal: Fix Filter Click
**Status: Ready for Review**

## Intent
Eliminate the audible click that occurs when the filter offset changes. The click arises because the biquad state is zeroed on each coefficient update, creating a step discontinuity in the output signal.

## Specification Deltas

### MODIFIED
- Filter offset changes produce no audible click or discontinuity. The transition between filter states is inaudible under normal use.

## Scope
- **In scope**: click-free filter transitions on step and reset.
- **Out of scope**: smooth parameter interpolation (gliding cutoff), crossfade between filter states.
