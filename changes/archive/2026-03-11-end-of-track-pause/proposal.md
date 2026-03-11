# Proposal: End-of-Track Pause
**Status: Ready for Review**

## Intent
When playback reaches the end of a track, the player currently exits. It should instead stop playback and remain in the player view, leaving the track loaded and the UI interactive.

## Specification Deltas

### ADDED
- When playback reaches the end of the track, the transport pauses and the playhead remains at the end position. The player view stays open and fully interactive.

### MODIFIED
- Reaching the end of the track no longer exits the application.
