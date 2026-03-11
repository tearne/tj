# Design: HPF / LPF Filter
**Status: Draft**

## Approach

### Filter Algorithm

A second-order (biquad) Butterworth IIR filter will be implemented directly in the render/audio thread. The biquad difference equation is:

```
y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
```

Butterworth coefficients for LPF and HPF at normalised frequency `w0 = 2π·fc/fs`:

```
LPF:  b0 = (1-cos w0)/2,  b1 = 1-cos w0,  b2 = (1-cos w0)/2
HPF:  b0 = (1+cos w0)/2,  b1 = -(1+cos w0), b2 = (1+cos w0)/2

Shared denominator:
  a0 = 1 + sin(w0)/sqrt(2)
  a1 = -2·cos(w0) / a0
  a2 = (1 - sin(w0)/sqrt(2)) / a0
  b0/=a0, b1/=a0, b2/=a0
```

The filter is applied per-channel after rodio decodes each frame.

### Cutoff Frequency Mapping

`filter_offset` steps map logarithmically:

```
offset  ±1  →  ~18 kHz / ~40 Hz   (barely audible)
offset ±10  →  ~40 Hz  / ~18 kHz  (fully cut)
```

Concrete mapping (absolute value 1–10 → fc):
```rust
const CUTOFFS_HZ: [f64; 10] = [
    18000.0, 12000.0, 8000.0, 5000.0, 3000.0,
     1500.0,   700.0,  300.0,  100.0,   40.0,
];
// index = abs(filter_offset) - 1
// LPF uses CUTOFFS_HZ[idx], HPF uses CUTOFFS_HZ[9 - idx]
```

This gives a roughly log-spaced sweep from near-flat to fully cut at both ends.

### Integration with rodio

The audio source chain is:

```
SymphoniaDecoder → FilterSource (new wrapper) → mixer input
```

`FilterSource<S>` wraps any `rodio::Source` and holds:
- Biquad coefficients (`b0, b1, b2, a1, a2`)
- Per-channel state: `x1, x2, y1, y2` (two history samples each)
- An `Arc<AtomicI32>` for `filter_offset`, shared with the main thread

On each call to `Iterator::next()`, `FilterSource` reads the shared `filter_offset`. If it has changed since last read, recompute coefficients and reset state (`x1=x2=y1=y2=0` per channel). At `filter_offset == 0` the filter is bypassed (samples pass through unchanged).

### Key Bindings

| Key | Action |
|-----|--------|
| `[` | Decrease `filter_offset` by 1 (clamp at −10) |
| `]` | Increase `filter_offset` by 1 (clamp at +10) |
| `Space+[` or `Space+]` | Snap `filter_offset` to 0 (flat) |

**Conflict note**: The Playlists proposal also reserves `Space+[`/`Space+]` for prev/next track. Until playlists are implemented these keys are unambiguous. When playlists land, the flat-snap chord should move to a different binding (to be decided at that point). The design for this feature uses `Space+[`/`Space+]` = flat snap; no playlist navigation exists yet.

### Info Bar

When `filter_offset ≠ 0`, insert before the spectrum indicator:
```
lpf:3   (filter_offset < 0, show abs value)
hpf:5   (filter_offset > 0)
```
Rendered in a dim/cyan style consistent with other info-bar fields.

### State & Persistence

- `filter_offset: i32` — local variable in the main event loop, initialised to 0.
- Not persisted (sidecar unchanged).
- No new config keys.

---

## Tasks

1. ✓ **Impl**: Add `FilterSource<S>` wrapper struct implementing `rodio::Source` + `Iterator`, with biquad per-channel state and shared `Arc<AtomicI32>` for `filter_offset`.
2. ✓ **Impl**: Insert `FilterSource` into the audio source chain (between decoder and mixer); store the shared `Arc<AtomicI32>` in the main loop.
3. ✓ **Impl**: Add `FilterDecrease` and `FilterIncrease` actions; bind `[` / `]` in `resources/config.toml`; update the action handler.
4. ✓ **Impl**: Add `FilterReset` action; bind `Space+[` and `Space+]` in `resources/config.toml`; update the action handler.
5. ✓ **Impl**: Render filter indicator (`lpf:N` / `hpf:N`) in the info bar.
6. **Verify**: Manual test — sweep `[`/`]` while music plays; confirm audible LPF/HPF effect, flat bypass at 0, info bar updates correctly.
7. **Process**: Confirm ready to archive.
