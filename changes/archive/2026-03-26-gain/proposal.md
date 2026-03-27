# Gain Trim Proposal
**Type**: Proposal
**Status**: Approved

## Overview

Add a per-deck gain trim control (`J`/`M` for Deck 1, `K`/`,` shifted equivalents for Deck 2) that scales the signal pre-fader. This lets quiet tracks be boosted and loud ones attenuated so that level 100% represents the right mix position for every track, independent of the source recording level.

## Keys

| Key | Action |
|-----|--------|
| `J` (Shift+j) | Deck 1 gain +1 dB |
| `M` (Shift+m) | Deck 1 gain −1 dB |
| `K` (Shift+k) | Deck 2 gain +1 dB |
| `<` (Shift+,) | Deck 2 gain −1 dB |

These sit naturally on the same physical keys as the existing level controls (`j`/`m`, `k`/`,`), with Shift selecting the gain layer.

## Signal Chain Placement

Gain is applied inside `FilterSource::next()` to the filtered sample, before PFL routing and before the player volume is applied:

```
raw sample → filter → × gain → PFL routing → × fader volume → output
```

This means the gain affects both the main mix and the PFL monitor uniformly, which is the correct behaviour for a trim control.

## Range and Step

- Range: −12 dB to +12 dB (linear: ~0.251 to ~3.981)
- Step: 1 dB (linear multiplier: 10^(1/20) ≈ 1.122)
- Default: 0 dB (linear 1.0)

A ±12 dB window covers most real-world track level variation without creating clipping risk at the top or making the control feel sluggish.

## State

`gain_db: i8` stored on `Deck` (integer dB, range −12..=12). The corresponding linear multiplier `gain_linear: f32` stored as `Arc<AtomicU32>` (f32 bits) on `DeckAudio`, mirroring the existing pattern for `deck_volume_atomic`.

## Display

A single character is appended immediately after the closing bracket of the level indicator (no label, always visible). The `▁▂▃▄▅▆▇` set is used: 7 characters mapped uniformly across -12..+12 dB, so `▄` (half-fill) lands exactly at 0 dB. Characters below `▄` indicate negative gain; above indicate positive. Coloured dim amber matching the surrounding info bar — not highlighted unless changed.

## Persistence

Gain is stored in the per-track cache entry alongside BPM, offset, and cue — same Blake3-keyed JSON structure. It resets to 0 dB on a fresh track with no cache entry.
