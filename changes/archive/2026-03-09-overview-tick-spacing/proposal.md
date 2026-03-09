# Proposal: Overview Tick Spacing and Waveform View Terminology
**Status: Draft**

## Intent

Two related concerns:

1. The Overview's bar markers can become illegible at narrow terminal widths or slow tempos, where adjacent tick columns are less than one character apart and markers visually merge into a continuous line.
2. The two waveform views lack formally defined names, leading to inconsistent language ("detail", "zoomed", "zoomed-in") across the spec.

## Specification Deltas

### ADDED

- **Waveform view names**: The two waveform views are formally named:
  - **Overview** — the full-track waveform with a playhead marker showing current position.
  - **Detail view** — the zoomed waveform centred on the playhead, with variable zoom level.
  These names are used consistently throughout the specification and UI.

- **Overview tick legend**: The Overview displays a legend in its top-right corner indicating the current bar interval between tick markers (e.g. `4 bars` or `8 bars`).

### MODIFIED

- **Overview tick spacing**: The Overview displays bar markers at a minimum interval of 4 bars. If the column distance between adjacent markers is less than 2 characters (i.e. no blank character gap exists between them), the interval is doubled repeatedly until a gap of at least 1 character exists between all marker positions. The legend updates to reflect the current interval.
