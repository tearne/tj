# Proposal: Info Bar Layout Revision
**Status: Note**

## Intent
The info bar is a single left-aligned line of variable-width spans. When any field changes width — BPM gaining a speed adjustment in parentheses, offset growing an extra digit, tap count appearing — every field to its right shifts horizontally. The spectrum strip is particularly affected: it visibly jumps left and right during normal use, which is distracting.

A revised layout should:
- Keep the spectrum strip (and other stable elements) at a fixed horizontal position.
- Remove or relocate fields that do not need to be visible at all times.

## Unresolved
- Which fields are truly necessary at all times vs. transient or removable?
  - Candidates for removal or relocation: palette name, nudge mode (already shown in detail panel?), zoom level, volume.
  - Candidates that must stay: BPM, play state, filter indicator.
- Fixed-width approach vs. right-anchoring the spectrum: does the terminal width make right-anchoring reliable?
- Should transient state (tap count, filter indicator, latency) appear as overlays or badges rather than inline fields?
