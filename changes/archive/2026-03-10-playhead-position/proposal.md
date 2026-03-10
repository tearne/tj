# Proposal: Configurable Playhead Position
**Status: Ready for Review**

## Intent
The detail waveform playhead is currently fixed at the horizontal centre of the panel. Moving it to 20% from the left provides more lookahead — the track ahead of the current position fills most of the screen, which is more useful during playback. The position should be user-configurable.

## Specification Deltas

### ADDED
- A `playhead_position` parameter under a `[display]` section (integer, 0–100) sets the playhead's horizontal position as a percentage of the detail panel width. Default: `20`.
- If no config file exists, the embedded default (which includes `[display]`) is written as before. No modifications are made to existing config files.

### MODIFIED
- The detail waveform playhead moves from the fixed centre column to the column nearest `playhead_position`% of the panel width from the left edge.
- All detail-panel rendering derived from the playhead column (`centre_col`, buffer anchor, tick mark origin) updates to use the configured position.
- Values outside 0–100 are clamped silently to the nearest bound.
