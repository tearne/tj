# UI Chrome Theming
**Type**: Fix
**Status**: Done

## Problem

Several UI chrome elements use neutral grays that sit outside the warm spectral
palette used by the waveforms and beat indicators, giving the interface an
inconsistent feel:

- **Track name** (notification line): `spectral_color(palette, 0.0, 0.55)` —
  correct palette colour but too dim to read comfortably.
- **Detail info bar** (zoom / latency / nudge / palette line): `Rgb(60, 60, 60)` —
  neutral gray, no palette tint.
- **Global bar** (idle state, showing browser directory): `Rgb(50, 50, 50)` —
  neutral gray, no palette tint.

## Fix

### Track name
Increase the brightness argument from `0.55` toward `0.85` (exact value to be
confirmed visually). Must remain clearly dimmer than high-priority notifications
(BPM pending, active_notification), which are styled independently in yellow/red.

### Detail info bar and global bar
Replace the neutral gray with a warm-tinted dim color derived from the spectral
palette — `spectral_color(palette, 0.0, <low brightness>)` — so all three chrome
bars share the same hue family. The detail info bar and global bar carry secondary
information and should stay subdued; a brightness of around `0.25`–`0.35` is the
target range.

The global bar's idle content (browser directory path) and the detail info bar's
content (zoom, latency, nudge mode, palette name) are both informational, so the
same brightness level suits both.

## Log

Track name brightness raised from 0.55 → 0.85. Detail info bar and global bar
diverged from the original plan (warm spectral tint) — visual review showed
`Color::DarkGray` matched the deck info/notification bars better. Global bar
also gained the `Rgb(20, 20, 38)` background to match the deck notification bar
style.
