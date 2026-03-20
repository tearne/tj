# Note: Display Position Invariants

Two implicit contracts in the rendering/offset code have been violated more than once.
Neither is documented anywhere, which means they must be re-discovered each time the
relevant code is touched.

## Invariant 1 — Tick/cue rendering uses exact position; waveform buffer uses quantized position

The waveform buffer is rendered at discrete half-column-aligned anchor positions. The
display position (`smooth_display_samp`) is mapped to a buffer column by rounding to
the nearest half-column (`delta_half = round(delta / half_col_samp)`). This quantization
is correct for waveform scrolling — the buffer grid is discrete.

Tick and cue marker rendering is different: each marker is placed at a continuous sample
position, and its screen column is computed by dividing the distance from `view_start` by
`half_col_samp` and rounding. If `view_start` is derived from the quantized buffer anchor
rather than the exact `smooth_display_samp`, the quantization residual can shift a tick's
`disp_half` by 1 — causing a half-character wobble whenever the display position moves.

**The rule**: `view_start` for tick/cue rendering must be derived from the exact
`display_pos_samp`, not from `anchor_sample + delta_half * half_col_samp`.

**Violated in**: tick wobble bug during the `key-direction-audit` experiment (v0.5.107).
Fixed by computing `detail_view_start = display_pos_samp - centre_col * samples_per_col`.

**Where this lives**: `render_detail_waveform` in `main.rs`.

## Invariant 2 — Offset-step display delta is always the raw step, not the arithmetic difference

When `offset_ms` is adjusted by ±10ms and the track is paused, `smooth_display_samp`
is shifted by the same sample count so that ticks appear stationary (the waveform moves
instead). The delta must be the raw keypress step (±10ms), not `new_offset - old_offset`.

These differ when `offset_ms` wraps via `rem_euclid`: for example, subtracting 10ms from
an offset of 5ms wraps to ~490ms (one beat period minus 5ms). Using `new - old` gives a
~480ms shift in `smooth_display_samp`, which moves the audio position far enough to
trigger the background renderer's rerender threshold (`drift >= cols * 3/4`), causing a
full waveform redraw.

**The rule**: in offset handlers, hardcode `±10.0 / 1000.0 * sample_rate` as the display
delta; never derive it from the post-`rem_euclid` offset values.

**Violated in**: waveform rerender bug during the `key-direction-audit` experiment (v0.5.107).
Fixed in v0.5.108 by using the literal ±10ms delta.

**Where this lives**: the four offset handlers (`Deck1/2 OffsetIncrease/Decrease`) in `main.rs`.

## Structural improvements that would help

Both violations happened because the invariants are implicit and the relevant code is
repeated (four offset handlers, one shared view_start for conceptually different uses).

**Option A — Separate the two view_start computations explicitly**

Name `tick_view_start` and `waveform_viewport_start` as distinct values in
`render_detail_waveform`, with a comment on each explaining which position it uses
and why. This makes Invariant 1 visible at the call site rather than hidden in a formula.

**Option B — Extract `apply_offset_step(d, delta_ms: i64)`**

A single helper that applies the offset change, computes the display delta from the
raw step, and updates `smooth_display_samp` and the seek position. The four handlers
call it with `+10` or `-10`. This makes Invariant 2 impossible to violate per-handler.

Either option could be a small proposal if we decide to act on it.
