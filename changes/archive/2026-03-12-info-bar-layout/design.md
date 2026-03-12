# Design: Info Bar Layout Revision
**Status: Draft**

## Approach

### Layout split

The info bar becomes two groups separated by a spacer:

```
⏸  120.00 (+124.40)  +7ms  tap:3       nudge:jump  zoom:4s  level:80%  [?]  lpf:3  ▕⣿⣿⣿⣿⣿⣿⣿⣿▏
└─────────── left ──────────────────┘   └────────────────── right ──────────────────────────────┘
```

The spacer width = `bar_width − left_width − right_width` (minimum 1). Since the right group is built first and its width is known, the spacer fills whatever remains. The spectrum is always the rightmost element, pinned to the terminal edge.

**Left group**: play icon · BPM · phase offset · tap count (transient) · calibration text (transient)
**Right group**: nudge active indicator (transient) · `nudge:jump/warp` · `zoom:Ns` · `level:N%` · `[?]` · filter indicator (transient) · spectrum

### Nudge fixed width

`"jump"` and `"warp"` are both 4 characters — `nudge:jump` / `nudge:warp` are already the same width. No padding needed.

### Zoom label

`zoom_secs` displayed as `zoom:4s` (was bare `4s`).

### Volume → level

- Display: `level:N%` (was `vol:N%`)
- Config keys: `level_up` / `level_down` (was `volume_up` / `volume_down`)
- Action enum variants kept as `VolumeUp` / `VolumeDown` (internal only, no user-visible change)
- `ACTION_NAMES` entries updated to `"level_up"` / `"level_down"`

### Palette name removed

Remove `SPECTRAL_PALETTES[palette_idx].0` from the info bar string.

### Calibration mode

Calibration text (`lat:Nms  ~ to exit`) moves to the left group (appended after offset), replacing the right group entirely during calibration (spectrum already hidden in calibration mode).

## Tasks

1. ✓ **Impl**: Rename config keys `volume_up`/`volume_down` → `level_up`/`level_down` in `resources/config.toml` and `ACTION_NAMES`.
2. ✓ **Impl**: Rebuild the info bar as left + spacer + right groups; update all field labels (`zoom:`, `level:`, remove palette name).
3. **Verify**: Confirm spectrum stays pinned right while BPM adjustment, tap count, and filter indicator appear/disappear.
4. **Process**: Confirm ready to archive.
