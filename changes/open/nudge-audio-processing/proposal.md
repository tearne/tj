# Proposal: Nudge Audio Processing
**Status: Draft**

## Intent
When paused and c/d is used to nudge, the scrub audio plays back without the active filter or level settings applied. Scrub snippets should sound the same as normal playback — i.e. pass through the same filter and level processing.

## Specification Deltas

### MODIFIED
- Scrub audio fired during paused nudge must respect the active filter setting (LPF/HPF cutoff offset) and the current level (volume), matching the audio processing applied during normal playback.
