# Mixer

The mixer addresses each deck directly via fixed key columns, independent of the selected deck. Mixer controls (level, gain, filter, PFL) may be adjusted on either deck at any time.

## Level

- Each deck has a fader level in [0.0, 1.0] applied to the audio output after gain and PFL routing.
- `j` / `m` (Deck 1) and `k` / `,` (Deck 2) increase / decrease level by 0.05. Clamps silently at 0.0–1.0.
- `Space+J` / `Space+M` snap Deck 1 level to 1.0 / 0.0; `Space+K` / `Space+,` do the same for Deck 2.

## Gain Trim

- Each deck has an independent gain trim applied to the audio signal after the filter and before the fader. The trim range is ±12 dB in 1 dB steps.
- `J` / `M` (Deck 1) and `K` / `<` (Deck 2) increase / decrease gain by 1 dB. Clamps silently at ±12 dB.
- Gain is applied as a linear multiplier (`10^(dB/20)`) in the audio signal chain, after the filter and before PFL routing.
- Gain is persisted to the cache alongside BPM and offset, and restored when the track is loaded.
- The detail info bar shows a single character gain indicator immediately after the level closing bracket. It uses `▁▂▃▄▅▆▇` to represent the range −12 dB to +12 dB, with `▄` at 0 dB. The indicator is grey at 0 dB and dim amber at any non-zero value.

## Filter

- Each deck has a sweepable filter with a flat (bypass) centre position. The filter offset is an integer in [−16, +16]: positive values move toward HPF, negative toward LPF.
- `7` / `u` (Deck 1) and `8` / `i` (Deck 2) increase / decrease the filter offset by 1. `Space+7` or `Space+u` reset Deck 1 to flat; `Space+8` or `Space+i` reset Deck 2.
- Filter slope is switchable between 12 dB/oct (2-pole) and 24 dB/oct (4-pole). `&` / `U` (Deck 1) and `*` / `I` (Deck 2) step the slope up / down.
- The filter is a Butterworth biquad applied in the audio signal chain before gain and PFL routing.
- The spectrum analyser display reflects the active filter.

## PFL Monitor

- PFL (Pre-Fader Listen) routes the selected deck's audio to the left channel of the output for headphone cueing.
- The PFL level is a float in [0.0, 1.0] stored as a u8 value 0–100 internally. It is adjusted in steps of 0.20 (20 units): `s` / `x` increase / decrease; `Space+S` or `Space+X` reset to 0.
- `Space+G` toggles PFL on/off for the selected deck: when toggled on the level is set to 1.0 (100); when toggled off the level is set to 0.
- Only one deck can be in PFL at a time. Activating PFL for the selected deck clears any PFL on the other deck.
- The PFL level is applied as a scale factor in the audio signal chain on the left channel when PFL is active.
