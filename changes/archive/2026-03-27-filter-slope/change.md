# Filter Slope
**Type**: Proposal
**Status**: Done

## Log
- Added dB/oct indicator to info line: appears to the right of the spectrum analyser when a filter is active, reserves fixed space when inactive to prevent reflow.
- Added variable filter slope: `&`/`U` (deck 1) and `*`/`I` (deck 2) toggle between 12 dB/oct (2-pole) and 24 dB/oct (4-pole). Implemented as two cascaded Butterworth biquad stages; second stage active when `filter_poles == 4`. dB/oct indicator updates dynamically.
