# Design: Overview Spectral Colour
**Status: Approved**

## Approach

### Spectral estimation
For each chunk of N mono samples, compute:
- `total_energy = mean(s[i]²)`
- `diff_energy  = mean((s[i+1] − s[i])²)`

For a pure sine at frequency f, `sqrt(diff / total) ≈ 2π·f/fs`. This gives a per-chunk spectral rate in radians/sample proportional to the dominant frequency.

Define the crossover at **250 Hz** (bass_ratio = 0.5):

```
k = 2.0 × 2π × 250 / sample_rate   // ≈ 0.0712 at 44100 Hz
bass_ratio = (1.0 − sqrt(diff / (total + ε)) / k).clamp(0, 1)
```

- bass_ratio = 1.0 → energy concentrated below 250 Hz (bass/DC)
- bass_ratio = 0.5 → midpoint ~250 Hz (crossover)
- bass_ratio = 0.0 → energy above ~500 Hz (treble)

Computed once at load time in `WaveformData::compute`, stored in a new `bass_ratio: Vec<f32>` field (parallel to `peaks`, same `OVERVIEW_RESOLUTION` length). The `sample_rate` parameter to `compute` (currently unused) is used here.

### Colour blend
Linear interpolation in RGB between:
- **Orange** (255, 140, 0) at bass_ratio = 1.0
- **Cyan**   (0, 200, 200) at bass_ratio = 0.0

```
R = (255.0 * r) as u8
G = (200.0 - 60.0 * r) as u8   // 140 → 200
B = (200.0 * (1.0 - r)) as u8  // 0   → 200
```

Midpoint (~250 Hz): RGB(128, 170, 100) — a muted yellow-green, visually between the two extremes.

### Overview render
The overview already maps screen columns to `waveform.peaks[idx]` via index arithmetic. The same `idx` is used to look up `waveform.bass_ratio[idx]`, giving a per-column `Color::Rgb`. This replaces the uniform `Color::Green` for waveform cells. Tick marks, playhead, and legend retain their existing colours.

### Colour palettes
Four palettes, cycled with `p`. Each is a (bass_colour, treble_colour) pair blended linearly by `bass_ratio`:

| # | Name | Bass | Treble |
|---|------|------|--------|
| 0 | Amber/Cyan | (255,140,0) | (0,200,200) |
| 1 | Soft | (200,130,50) | (50,190,200) |
| 2 | Spectrum | (80,110,220) | (220,200,60) |
| 3 | Green | (120,200,60) | (60,200,170) |

The active palette name is shown in the info bar (dim text). Palette selection is not persisted.

## Tasks
1. ✓ Impl: Add `bass_ratio: Vec<f32>` to `WaveformData`; compute it in `compute()` using the formula above
2. ✓ Impl: Overview render — look up `bass_ratio` per column and use `Color::Rgb` blend instead of `Color::Green`
3. ✓ Impl: Add `SPECTRAL_PALETTES` constant and `palette_idx` state; `p` key cycles through palettes; palette name shown in info bar
4. ✓ Verify: Overview shows colour variation; `p` cycles palettes visibly; info bar shows palette name
5. ✓ Process: confirm ready to archive
