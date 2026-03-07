# Proposal: Beat Jump
**Status: Ready for Review**

## Intent
Implement beat jump — jumping the playhead forward or backward by a configurable number of beats — with no perceptible gap, no noise artefacts, and correct rhythmic continuity.

## Specification Deltas

### ADDED

**Beat Jump behaviour**:
- `[` jumps backward and `]` jumps forward by the current beat unit.
- Beat units selectable with keys `1`–`6`: 4, 8, 16, 32, 64, 128 beats.
- The jump is by exactly N × beat_period seconds from the current position — no snapping to the beat grid, preserving rhythmic continuity.
- Jumping backward past the start of the track clamps to position 0.
- Jumping forward past the end of the track is a no-op.
- The current beat unit is shown in the UI.

**Seamless playback**:
- Seeking is implemented via an atomic position counter shared between the audio thread and the UI thread. The audio thread never pauses; it reads from the new position on the very next sample pull.
- To avoid click artefacts from cutting at a non-zero sample value, the actual seek target is snapped to the nearest zero crossing within a short window (~10ms) of the computed target position.

### MODIFIED
- **Player Controls** table: `[`/`]` and `1`–`6` rows were listed but unimplemented — now implemented.
- **Beat Jump** behaviour section: was already specified; add the zero-crossing snap and seamless playback constraints.
- Position display and waveform playhead derive from the atomic sample counter rather than `player.get_pos()`, so they remain accurate after a seek.
