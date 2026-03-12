# Proposal: Filter Visual Indicator + Level Bar
**Status: Ready for Review**

## Intent
Make the filter state visible within the spectrum strip, compress the level display, and remove the separate filter text indicator.

## Specification Deltas

### MODIFIED
- **Level indicator**: `level:N%` replaced by a single Unicode lower-eighth-block character (▁▂▃▄▅▆▇█) quantised across 0–100% in 8 steps. Label removed; the character alone conveys the value.
- **Spectrum strip**: doubled in width from 8 to 16 characters (32 frequency bins). When a filter is active, bins in the attenuated region are rendered with a grey background:
  - LPF active: bins to the right of the cutoff bin have a grey background.
  - HPF active: bins to the left of the cutoff bin have a grey background.
  - Flat (offset 0): no shading — spectrum renders as normal.
  - The cutoff bin boundary is derived from the active `filter_offset` mapped to the same frequency grid as the spectrum bins.

### REMOVED
- `lpf:N` / `hpf:N` text indicator from the info bar.
