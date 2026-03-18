# Proposal: Split SPEC.md into SPEC/ folder
**Status: Draft**

## Problem

`SPEC.md` has grown to ~340 lines covering eight distinct concerns in a single file. It is increasingly hard to navigate and will become harder to maintain as the application grows.

## Proposed Change

Replace `SPEC.md` with a `SPEC/` folder containing eight focused documents:

| File | Covers |
|---|---|
| `overview.md` | What tj is; launching; file browser behaviour; constraints; out of scope |
| `keymap.md` | ASCII keyboard layout diagram; modifier convention; key string format; config loading |
| `layout.md` | UI section order; responsive compression; info bar; notification bar; global status bar; empty deck panels |
| `waveforms.md` | Overview and detail waveforms; braille rendering pipeline; spectrum analyser; glossary |
| `transport.md` | Playback; deck selection; beat detection and BPM correction; tap BPM; beat jump; nudge; needle drop; metronome |
| `audio.md` | Level control; HPF/LPF filter; audio latency calibration |
| `architecture.md` | Threading model; caching; dependencies |
| `verification.md` | Verification scenarios |

The ASCII keyboard layout diagram is sourced from `changes/archive/2026-03-16-keymap-redesign/proposal.md` and placed in `keymap.md`.

## Effect

- `SPEC.md` is deleted.
- `SPEC/` contains eight files; all content is preserved, only reorganised.
- No code changes.

## Risk

None. Documentation only.
