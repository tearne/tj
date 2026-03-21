# Design: Display Position Invariants
**Status: Approved**

*(Retrospective — describes what was implemented.)*

## Approach

Two structural changes, one per invariant.

**Option A — Separate marker and waveform view-start (Invariant 1)**

In `render_detail_waveform`, rename `detail_view_start` to `marker_view_start` and
compute it directly from `display_pos_samp`. The existing `viewport_start` (quantized
via `delta_half`) is unchanged — it is correct for discrete buffer-column lookup. The
comment at `marker_view_start` names the contrast explicitly so the distinction is
visible at the call site.

**Option B — `apply_offset_step` helper (Invariant 2)**

Extract a free function `apply_offset_step(d: &mut Deck, delta_ms: i64)` that:
1. Applies `delta_ms` to `d.tempo.offset_ms` and wraps via `rem_euclid`.
2. If paused, shifts `smooth_display_samp` by the exact `delta_ms` sample count
   and calls `set_position` to match.

The four offset handlers (`Deck1/2 OffsetIncrease/Decrease`) each reduce to a single
call. The invariant (raw step, not post-wrap difference) is enforced in one place.
Placed alongside `anchor_beat_grid_to_cue` — the natural home for `&mut Deck` helpers.

## Tasks

1. ✓ Impl: rename `detail_view_start` → `marker_view_start`; update comment to name
   contrast with `viewport_start`
2. ✓ Impl: extract `apply_offset_step(d, delta_ms)`; replace four handler bodies
3. ✓ Verify: `cargo build` clean
