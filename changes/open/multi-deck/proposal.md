# Proposal: Multi-Deck Architecture
**Status: Note**

## Intent

Extend the player to support two independent decks for DJ-style mixing. Each deck has its own transport, BPM, filter, level, and file browser. The layout separates detail waveforms (top) from control elements (bottom), matching a typical DJ setup where the waveforms are above the controls.

## Conceptual Model

A **deck** comprises two distinct UI elements:
- **Detail element**: the detail waveform panel
- **Control element**: track name bar + info bar + overview waveform

When duplicated, these elements are not necessarily adjacent — detail waveforms of both decks appear together at the top; control elements appear together underneath.

## Intended Behaviour (from todo.md)

1. Layout: detail waveform of deck 1, detail waveform of deck 2, control of deck 1, control of deck 2 (top to bottom).
2. `g` selects deck 1 control zone; `h` selects deck 2 control zone. The selected deck receives all keyboard input (transport, level, filter, file browser, etc.). The active deck is visually highlighted.
3. On load, only one deck is active. The user selects a deck and opens the file browser to load a track.

## Prerequisites

- Code-review structural findings S1–S4 (module boundaries, `PlayerState`/`DisplayState` decomposition, `SeekHandle` consolidation) must be addressed first — duplicating the current monolith would double all existing structural debt.
- `track-name-infobar` (track name bar becomes the top of the control element).

## Unresolved

- Mixer / crossfader: is a hardware or software crossfader in scope?
- Shared vs independent audio latency calibration.
- How does the file browser attach to a deck — does it open within the deck's control zone or full-screen?
- Minimum terminal size requirements for two-deck layout.
