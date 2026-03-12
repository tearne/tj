# Design: Filter Visual Indicator + Level Bar
**Status: Draft**

## Approach

### Level bar character
Replace `level:N%` with a single character from `▁▂▃▄▅▆▇█` (U+2581–U+2588). Map 0–100% across 8 steps: 0%=`▁`, 12–25%=`▂`, …, 88–100%=`█`. The label is dropped entirely; the character stands alone.

### Spectrum width
`compute_spectrum` currently returns `([char; 8], [bool; 8])` over 16 bins. Doubling to 16 characters requires 32 bins. Changes:
- `freqs` array: `[f64; 32]`, log-spaced 20 Hz – 20 kHz.
- `heights` / `raw_heights`: `[usize; 32]` / `[f32; 32]`.
- Return type: `([char; 16], [bool; 16])`.
- `LEFT_MASKS` / `RIGHT_MASKS` logic unchanged (pairs of bins → one char).
- `BG_THRESH` accumulator arrays in tui_loop: `[bool; 16]`.
- Info bar rendering loop: `0..16`.

### Filter shading
Map `filter_offset` to a cutoff bin index within the 32-bin grid:
- The 32 bin frequencies are the same `freqs` array used by Goertzel.
- `FILTER_CUTOFFS_HZ` gives the cutoff frequency for the active offset.
- Find the bin index where `freqs[k]` crosses the cutoff: first bin above cutoff (LPF) or last bin below cutoff (HPF).
- Convert bin index to character index: `char_idx = bin_idx / 2`.
- At render time, each of the 16 characters gets one of three styles:
  - **In passband**: normal amber style (as today).
  - **In stopband**: grey background (`Color::Rgb(40, 40, 40)`), dim yellow fg if dots present.
  - **Flat**: no shading (all characters use normal style).

### Info bar width accounting
The spacer calculation uses `.chars().count()` on span content. Since the spectrum now emits 16 individual `Span`s of one char each (plus the two bracket spans), the count is still correct.

## Tasks

1. ✓ **Impl**: Replace `level:N%` with single eighth-block character in info bar rendering.
2. ✓ **Impl**: Expand `compute_spectrum` to 32 bins / 16 chars; update all dependent array sizes and the render loop.
3. ✓ **Impl**: Add filter shading — compute cutoff char index from `filter_offset`; apply grey background style to stopband characters in the render loop. Remove `lpf:N`/`hpf:N` span from info bar.
4. **Verify**: Level bar shows correct step at 0%, 50%, 100%. Spectrum renders at double width. LPF/HPF shading moves correctly as filter steps change. Flat shows no shading.
5. **Process**: Confirm ready to archive.
