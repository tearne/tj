# Design: End-of-Track Warning
**Status: Approved**

## Approach

### Beat parity
The existing beat phase (`smooth_pos_ns` modulo `beat_period`) already drives the info bar flash. For the warning, divide the beat index by 2 and check its parity: even = markers visible, odd = markers hidden (or vice versa). Beat index = `(smooth_pos_ns - offset_ns) / beat_period_ns` as an integer.

### Threshold check
`remaining = total_duration - pos` (using the real audio position). Warning is active when `!player.is_paused() && remaining < warning_threshold`.

### Colour
When warning is active and beat parity is "on": render bar markers in a muted reddish-grey (e.g. `Color::Rgb(120, 60, 60)`). When "off": skip drawing them entirely (same as normal non-warning rendering but markers simply absent). When warning is inactive: normal rendering.

### Config
Add `warning_threshold_secs: f32` to `DisplayConfig` (default `15.0`). Expose as `warning_threshold_secs = 15` in `[display]` section of the default `config.toml`.

## Tasks

1. ✓ **Impl**: Add `warning_threshold_secs` to `DisplayConfig` and default config.
2. ✓ **Impl**: Compute `warning_active` and `warn_beat_on` (beat-parity toggle) each frame from remaining time and beat index.
3. ✓ **Impl**: Pass warning state into the overview rendering and apply the reddish-grey colour / suppression to bar markers.
4. ✓ **Verify**: Markers flash in beat time during final 30s; inactive while paused; configurable threshold works.
5. **Process**: Archive
