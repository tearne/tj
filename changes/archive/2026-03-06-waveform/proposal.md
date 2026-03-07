# Proposal: Waveform Visualisation
**Status: Approved**

## Intent
Add the two waveform views described in the spec, making the player genuinely useful and providing visual context for navigation.

## Specification Deltas

No spec changes — this implements existing spec requirements:

> Two waveform views are displayed simultaneously:
> - **Overview**: full-track waveform, with a playhead marker showing current position.
> - **Detail**: zoomed-in waveform centred on the playhead, with variable zoom level.
> Both views update in real time during playback.

## Scope
- **In scope**: overview waveform, detail waveform, playhead marker, zoom control on detail view.
- **Out of scope**: colours/theming, stereo (mono mix used for display), seek via clicking.
