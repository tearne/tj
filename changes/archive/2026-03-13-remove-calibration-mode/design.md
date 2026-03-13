# Design: Remove Calibration Mode
**Status: Approved**

## Approach

All calibration-mode state and behaviour is self-contained. Removal is a
straightforward deletion pass through `src/main.rs` and `resources/config.toml`.

Affected areas:

- **State variables** (`main.rs`): `calibration_mode: bool`, `pre_calibration_zoom_idx`, `last_calib_pulse: Option<Instant>`
- **Pulse firing loop** (~line 588): the `if calibration_mode` block that fires a click at 120 BPM
- **Metronome guard** (~line 603): `&& !calibration_mode` condition — remove the guard
- **Spectrum guard** (~line 633): `if !calibration_mode` condition — remove the guard
- **Info bar left group** (~line 745): the `if calibration_mode { … } else { … }` branch — keep only the else body
- **Info bar right group** (~line 777): `else if !calibration_mode` condition — simplify to always render the right group (except during pending_bpm, which is unchanged)
- **Detail waveform render** (~lines 1019–1102): `calib_display`, `calib_pulse_on_playhead` locals and the `if calibration_mode` blanking/marker block
- **Key binding help text** (~line 1177): remove the `~` calibration line; update `[`/`]` description (drop "outside calibration mode")
- **`d`/`c` latency-adjust guards** (~lines 1282, 1308): remove the `if calibration_mode` branches that routed `d`/`c` to latency adjustment
- **`CalibrationToggle` action handler** (~line 1487): remove the entire `Some(Action::CalibrationToggle)` arm
- **Zoom guards** (~lines 1561, 1564): remove `&& !calibration_mode` from zoom in/out
- **`Action::CalibrationToggle` variant** (~line 1717): remove from enum
- **`ACTION_NAMES`** (~line 1765): remove `("calibration_toggle", Action::CalibrationToggle)` entry
- **`config.toml`**: remove `calibration_toggle = "~"` line

## Tasks

1. ✓ **Impl**: Remove all calibration-mode state, logic, and rendering from `src/main.rs`
2. ✓ **Impl**: Remove `calibration_toggle` from `resources/config.toml` and the help text
3. ✓ **Process**: Build clean — ready to archive
