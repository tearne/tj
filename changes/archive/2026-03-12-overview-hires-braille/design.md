# Design: Overview Half-Column Braille Resolution
**Status: Draft**

## Approach

`render_braille` encodes each peak into a full braille character (left + right dot columns show the same data). By sampling `ow × 2` peaks and packing adjacent pairs into single characters — left dot column from even peak, right dot column from odd peak — the overview gets twice the horizontal audio resolution at zero extra screen width.

**Combining step** (after calling `render_braille` with `cols = ow × 2`):

```
combined[c] = (hires[c×2] & 0x47) | (hires[c×2+1] & 0xB8)
```

- `0x47` = `0b01000111` masks the four left-column dot bits (0, 1, 2, 6)
- `0xB8` = `0b10111000` masks the four right-column dot bits (3, 4, 5, 7)

The `ov_bass` colour value for each output column is the average of its two input half-columns. Everything else (playhead, tick marks, legend, colour loop) is unchanged — all operate in screen-column space.

## Tasks

1. **Impl**: Sample `ow × 2` peaks and bass values; call `render_braille` at double width; pack adjacent column pairs with the bitmask combine into `ow` bytes per row; average bass pairs for colour.
2. **Verify**: Visual check — overview should show the same waveform shape with visibly finer horizontal detail.
3. **Process**: Confirm ready to archive.
