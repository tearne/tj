# Filter Click
**Type**: Fix
**Status**: Done

## Problem
Audible click when engaging the filter (offset 0→nonzero), disengaging it (nonzero→0), and when switching filter poles (12→24 dB/oct or vice versa). Root causes:
- Engage: biquad starts from stale/zero state, initial output mismatches raw signal
- Disengage: instant switch from filtered output to raw signal
- Poles increase (2→4): stage 2 starts with zero state while stage 1 has accumulated history

## Fix
Replace the existing `output_fade_remaining` settle-fade with a unified crossfade mechanism (`last_y` + `transition_fade`) that blends from the previous output to the new output across all transition types:
- Engage: blend from last raw signal → new filtered output
- Disengage: blend from last filtered output → raw signal; zero biquad state on transition
- Poles 2→4: copy stage 1 state into stage 2 before engaging, then blend
- Poles 4→2: zero stage 2 state, then blend
- State reset (seek): same as before, blend from 0 → new filtered output

## Log
- Replaced `output_fade_remaining` with `last_y` (per-channel) + `transition_fade` crossfade mechanism in `FilterSource`.
- Engage (0→nonzero): starts crossfade from last raw output to new filtered output.
- Disengage (nonzero→0): zeroes biquad state, starts crossfade from last filtered output to raw.
- Poles increase (2→4): copies stage 1 state into stage 2 before activating, then crossfades — stage 2 starts close to steady state so the additional attenuation phase in smoothly.
- Poles decrease (4→2): zeroes stage 2 state, crossfades from last 4-pole output to new 2-pole output.
- State reset (after seek): zeroes `last_y` and biquad state, crossfades from silence to new filtered output.
