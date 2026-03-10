# Proposal: Dedicated Beat Jump Buttons
**Status: Approved**

## Intent
Replace the current beat jump model (select a unit with `1`–`7`, then jump with `[`/`]`) with eight dedicated actions — one per combination of direction and beat count. This eliminates the unit-selector state and makes each jump directly invocable.

## Specification Deltas

### ADDED
- **Dedicated beat jump actions** (8 total):

| Action | Beats |
|--------|-------|
| `jump_backward_1` | 1 beat back |
| `jump_forward_1` | 1 beat forward |
| `jump_backward_4` | 4 beats back |
| `jump_forward_4` | 4 beats forward |
| `jump_backward_16` | 16 beats back |
| `jump_forward_16` | 16 beats forward |
| `jump_backward_64` | 64 beats back |
| `jump_forward_64` | 64 beats forward |

### REMOVED
- The beat unit selector (`1`–`7` keys, selecting from 4/8/16/32/64/128/256 beats).
- The generic `beat_jump_backward` / `beat_jump_forward` actions (`[` / `]`).
- Display of the current beat jump unit in the info bar.

### MODIFIED
- Beat jump logic uses the beat count embedded in the action rather than a shared unit state variable.
- Jump behaviour is otherwise unchanged: exactly N × beat_period seconds from current position; clamp to 0 on backward overshoot; no-op on forward overshoot past end.

## Key Assignments (dev config)
The intended bindings follow the number row (forward) and QWERTY row (backward):

| Key | Action |
|-----|--------|
| `1` | `jump_forward_1` |
| `q` | `jump_backward_1` |
| `2` | `jump_forward_4` |
| `w` | `jump_backward_4` |
| `3` | `jump_forward_16` |
| `e` | `jump_backward_16` |
| `4` | `jump_forward_64` |
| `r` | `jump_backward_64` |

Note: `q` (currently quit) and `r` (currently BPM redetect) conflict with existing hard-coded bindings. They will be rebound in the dev config created during the keyboard-mapping change:
- `quit` → `esc`. Ctrl-C is hard-coded as an unconditional quit (conventional terminal behaviour) and is not configurable.
- `bpm_redetect` → `t`.

## Scope
- **In scope**: removing unit-selector state, adding 8 jump actions, updating the info bar.
- **Out of scope**: key assignments — those are defined in the dev config as part of the keyboard-mapping change.
