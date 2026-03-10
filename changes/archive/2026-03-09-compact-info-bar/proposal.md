# Proposal: Compact Info Bar
**Status: Approved**

## Intent
Consolidate all player status into a single compact information bar at the top of the player view, replacing the current separate BPM line and status/time line. The bar uses short symbolic indicators rather than verbose text, reducing visual clutter. A help popup (triggered by `h`) replaces the persistent key hints line, recovering a row of screen space.

The beat flash indicator moves onto the BPM value in the info bar — a soft yellow highlight rather than a separate panel — reducing visual distraction while preserving beat awareness.

## Specification Deltas

### MODIFIED
- **Player layout**: The BPM/offset line and the status/time line are replaced by a single info bar. The persistent key hints line is removed. The space recovered is added to the detail waveform area (or left blank).
- **Info bar**: A single line displaying (in order): play/pause state icon (`▶` or `⏸`), BPM value, phase offset, beat jump unit, zoom level, volume level, and a `[?]help` hint. The bar wraps gracefully if the terminal is too narrow. During BPM analysis the BPM field shows the animated spinner as before.
- **Beat indicator**: Removed as a separate panel. Instead, the BPM value in the info bar receives a soft yellow highlight for the duration of each beat flash window, matching the previous flash timing exactly.
- **Help popup**: Pressing `?` opens a modal overlay listing all key bindings. Pressing any key dismisses it. `h` retains its existing function (halve BPM).

### REMOVED
- Persistent key hints line at the bottom of the player view.
- Standalone beat indicator panel.
