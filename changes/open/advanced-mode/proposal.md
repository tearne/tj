# Proposal: Advanced Mode
**Status: Note**

## Intent
Provide an alternative operating mode for use when BPM detection is unreliable or irrelevant — e.g. non-rhythmic material, mixed content, or when the user simply wants time-based navigation without beat analysis interfering. In this mode, beat ticks are hidden and jump sizes are fixed time intervals derived from a standard 120 BPM grid rather than the detected tempo.

## Unresolved
- **Toggle**: how does the user enter/exit advanced mode? A dedicated key? A flag in config?
- **Jump sizes at 120 BPM**: 1/4/16/64 beats at 120 BPM = 0.5s / 2s / 8s / 32s. Are these the right time intervals, or should the user be able to configure the reference BPM?
- **What else changes**: does BPM analysis still run in the background (just hidden), or is it suppressed entirely? Does the beat flash still fire?
- **Name**: "advanced mode" is a placeholder — "time mode" or "free mode" may be more descriptive.
- **Persistence**: should the mode persist across tracks/sessions or reset on each load?
