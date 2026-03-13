# Proposal: Track Name Info Bar
**Status: Approved**

## Intent

Move the track name out of the terminal window title and TUI frame border, into a dedicated info bar line that sits alongside the existing info bar and overview waveform. This keeps it consistently visible within the TUI and positions it correctly as part of the control element in the future multi-deck layout.

## Specification Deltas

### ADDED

- A track name bar is displayed as a separate line within the control element, adjacent to the existing info bar and overview waveform. It shows the track name from metadata (artist – title if available, otherwise filename). Shown only when a track is loaded.

### MODIFIED

- The terminal window title is simplified to `tj vX.Y.Z` (no track name).
- The TUI frame border title is simplified to `tj vX.Y.Z` (no track name).

## Notes

- Exact position relative to the info bar and overview (above or below) TBD at design time.
- In the future multi-deck layout this bar travels with the control element.
