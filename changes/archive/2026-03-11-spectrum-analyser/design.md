# Design: Spectrum Analyser
**Status: Ready for Review**

## Approach

### Frequency analysis — Goertzel algorithm
No FFT library is required. The Goertzel algorithm computes the DFT magnitude at a single arbitrary frequency in O(N) time. Running it for 16 frequencies over a window of ~4096 samples is ~65 k operations per update — negligible at a 2×/beat update rate.

For each target frequency `f_k` and window of N samples `x[n]` (Hann-windowed):
```
coeff  = 2 * cos(2π * f_k / sample_rate)
s_prev2 = 0; s_prev = 0
for each sample x[n]:
    s = x[n] * hann[n] + coeff * s_prev - s_prev2
    s_prev2 = s_prev; s_prev = s
power = s_prev² + s_prev2² − coeff * s_prev * s_prev2
magnitude = sqrt(power)
```

### Frequency bins
16 logarithmically spaced centre frequencies from 20 Hz to 20 kHz:
```
f(i) = 20.0 × 1000^(i / 15.0)   for i = 0 … 15
```
Computed once at startup (or on first use) and stored as a `[f64; 16]` constant.

### Window
`N = 4096` samples taken from `mono` starting at `display_pos_samp`. The Hann window coefficients are pre-computed. Samples beyond the end of the track are treated as silence.

### Amplitude → dot height (0–4)
```
db = 20 * log10(magnitude / N)   (normalised magnitude)
height = clamp(round((db + 60) / 15), 0, 4)
```
This maps −60 dB (silence) → 0 dots and 0 dB (full-scale) → 4 dots, with 15 dB per dot row.

### Braille encoding
8 braille characters, each encoding two adjacent bins as a 2-column × 4-row bar chart.
Bars grow upward from the bottom row; each column uses 4 of the 8 braille dots.

```
Left-column masks  (dots 7,3,2,1): [0x00, 0x40, 0x44, 0x46, 0x47]
Right-column masks (dots 8,6,5,4): [0x00, 0x80, 0xA0, 0xB0, 0xB8]
byte = LEFT_MASKS[left_height] | RIGHT_MASKS[right_height]
char = '\u{2800}' | byte
```

### Update timing
State: `last_spectrum_update: Option<Instant>`, `spectrum_chars: [char; 8]` (held between updates).

Each frame, if `calibration_mode` is false and a track is loaded:
```
half_period = beat_period / 2          (or 500ms during analysis)
if elapsed since last update >= half_period:
    recompute spectrum_chars
    last_spectrum_update = now
```

### Rendering
Appended to the info bar as a dim-styled fixed-width span of 8 characters, preceded by a single space separator. Omitted (no span added) when the detail area width would push the info bar past the terminal width — ratatui clips naturally so this is graceful by default.

The Hann window coefficients and log-spaced frequencies are computed lazily on first use via a helper that returns them as `[f32; 4096]` and `[f32; 16]`.

## Tasks

1. ✓ **Impl**: Add `compute_spectrum` helper (Goertzel, Hann window, amplitude→dots, braille encoding).
2. ✓ **Impl**: Add `spectrum_chars` and `last_spectrum_update` state; drive recompute each frame.
3. ✓ **Impl**: Append spectrum span to info bar.
4. ✓ **Verify**: Spectrum responds visibly to music; updates twice per beat; holds during calibration.
5. ✓ **Process**: Archive.

