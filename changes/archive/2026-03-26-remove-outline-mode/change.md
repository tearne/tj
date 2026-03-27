# Remove Outline Mode
**Type**: Fix
**Status**: Approved

## Log
Removed the outline/fill toggle for the detail waveform (previously `O`/`waveform_style`). Fill mode is now the only mode. Removed: `WaveformStyle` action, keybinding, config entry, `style` atomic in `SharedDetailRenderer`, `outline` parameter from `render_braille`, and help text entry.
