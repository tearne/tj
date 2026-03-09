# Proposal: Beat Jump When Paused
**Status: Draft**

## Intent
Beat jump currently has no effect when the player is paused. This prevents using beat jump to navigate while cued up, which is a common DJ workflow.

## Specification Deltas

### MODIFIED
- **Beat Jump**: Beat jump operates regardless of whether the player is paused or playing. When paused, the playhead moves to the target position and remains paused.
