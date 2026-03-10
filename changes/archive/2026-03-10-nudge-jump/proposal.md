# Proposal: Micro-Jump
**Status: Approved**

## Intent
Add discrete micro-jump actions: each press moves the playhead forward or backward by 5ms. Key repeat fires continuously while held. Coexists with the existing hold-based speed-warp nudge on separate keys; the two approaches can be compared in practice.

## Specification Deltas

### ADDED
- `micro_jump_forward` — seeks playhead forward 5ms. Fires on press and repeat while held.
- `micro_jump_backward` — seeks playhead backward 5ms. Fires on press and repeat while held.

## Key Assignments (dev config)
- `micro_jump_forward` → `d`
- `micro_jump_backward` → `c`
