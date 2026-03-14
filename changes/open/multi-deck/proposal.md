# Proposal: Multi-Deck
**Status: Draft**

## Intent

Evolve tj from a single-track player into a two-deck DJ setup. Each deck is an independent playback unit with its own track, transport, BPM, waveform, and controls. A crossfader blends the audio output between the two decks. This is the 0.5.0 release.

## Specification Deltas

### ADDED

- **Two decks** — Deck A and Deck B — each capable of independently loading and playing a track. All per-track state (BPM, offset, volume, filter, metronome, zoom, waveform, tap accumulator, BPM analysis, notifications) is scoped to a deck.

- **Active deck** — one deck is active at a time and receives all keyboard input. `g` selects Deck A; `h` selects Deck B. The active deck is visually indicated.

- **Layout** — the UI is restructured into four vertical sections (top to bottom):
  1. Detail waveform — Deck A
  2. Detail waveform — Deck B
  3. Controls — Deck A (notification bar + info bar + overview)
  4. Controls — Deck B (notification bar + info bar + overview)
  - The active deck's control section is visually highlighted.

- **Loading a track** — the file browser loads a track into the active deck. The other deck continues uninterrupted.

### MODIFIED

- **Active-deck controls** (transport, BPM, offset, metronome, zoom, nudge, tap, re-detect) apply to the highlighted deck only, selected with `g` / `h`.

- **Per-deck fixed controls** — level and filter are bound to dedicated keys per deck, regardless of which deck is active, so both can be adjusted simultaneously without switching:
  - Deck A: `j`/`m` (level up/down), `u`/`7` (filter sweep)
  - Deck B: `k`/`,` (level up/down), `i`/`8` (filter sweep)

- **Global controls** (quit, help, terminal refresh, file browser open/close, deck select) are not deck-specific.

- **Audio latency** remains a single global setting shared across both decks.

## Scope

- **In scope**: two decks, deck selection, per-deck independent playback and level control
- **Out of scope**: crossfader / mixer (deferred — will be a subsequent change), more than two decks, per-deck EQ beyond the existing filter, hardware MIDI control, beat sync / auto-align between decks
