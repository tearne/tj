# Waveform Scroll Pulsation

## Intent

During playback the detail waveform scrolls with a visible rhythmic pulsation — alternating faster and slower — at roughly a 1-second period. The effect has been present since at least the playback-bugs commit and makes the display feel unsteady.

## Approach

`smooth_display_samp` advances using the nominal `frame_dur` each iteration. `output_position` advances at the real audio rate. Because `thread::sleep` overshoots, `output_position` consistently runs ahead, creating a steady-state lag that the `drift * 0.05` correction then has to fight. The correction factor gives a settling time constant of roughly one second, which amplifies any ~1Hz periodic perturbation in frame timing (CPU frequency scaling, OS scheduling) into visible scroll-speed oscillation.

Three options were explored, applied and tested one at a time.

**Option A — use elapsed for the smooth advance.** Replace `frame_dur.as_secs_f64()` with `elapsed` (already measured in the loop, capped at `frame_dur * 2.0` to absorb load spikes). This removes the systematic lag: smooth tracks the actual frame rate, so the correction term is near-zero in steady state and the time constant becomes irrelevant.

**Option B — increase the correction factor.** Change `0.05` to `0.12`. Simpler but does not address the underlying lag — ruled out when A was applied.

**Option C — raise the `large_drift` snap threshold.** Changed from `0.1` to `0.3`. Above typical accumulated drift but below a single beat at any practical BPM.

Options A and C were tested together and did not resolve the pulsation. The root cause was not in the position-advance formula.

---

The pulsation persists because the problem is in the render path, not the advance logic. The user's observation is the key: running two copies of the same track at slightly different offsets causes each deck to wiggle left-right independently, with the two decks alternating — a beat-frequency pattern consistent with two oscillators nearly in phase.

`render_detail_waveform` snaps the display position to the nearest half-column: `delta_half = (delta / half_col_samp).round()`. At standard zoom (col_secs ≈ 0.015s, 44100 Hz), half_col_samp ≈ 330 samples. `delta_half` oscillating between adjacent integers produces the visible left/right wiggle.

The oscillation source: the drift correction `smooth -= drift * 0.05` runs every frame. The audio device consumes samples in bursts (device callback), so `output_position` steps by J samples (typically 512–2048) between UI frames. When `output_position` steps by J, `drift` changes by J, and the correction perturbs `smooth` by J × 0.05. For a 2048-sample step: 2048 × 0.05 = 102 samples — within a factor of two of the 165-sample (half_col/2) rounding threshold, and the perturbation accumulates across frames before being cancelled. This is enough to toggle `delta_half` between N and N±1 on each audio device callback, producing the periodic wiggle.

With Option A applied (`smooth` advances by `elapsed`), the steady-state drift is near zero. The `× 0.05` correction now amplifies the audio-device step noise rather than correcting a genuine lag. The fix is to reduce the factor to near zero, relying on `large_drift` snapping for genuine divergences. At system-clock vs. audio-clock drift rates (sub-ppm), accumulated display error without correction is negligible — far below the 0.3s snap threshold.

**Option D (primary)**: Reduce the drift correction factor from `0.05` to `0.002`. This reduces the perturbation amplitude by 25×, well below the half-column visibility threshold. The `large_drift` snap at 0.3s remains the safety net for seeks and genuine divergence.

**Option E (if D insufficient)**: Pass `display_samp` as `f64` (not cast to `usize`) to `render_detail_waveform` and `extract_tick_viewport`. Use f64 delta in the half-column computation to eliminate the sub-sample truncation before rounding.

Review cadence: at the end.

## Plan

- [x] REVIEW `service_deck_frame` in `src/main.rs`: confirm `elapsed` is available at the smooth-advance call site and understand its current cap (`col_secs * 4.0` applied earlier in the loop); note whether `elapsed` already excludes the sleep time or includes it
- [x] UPDATE Option A — `src/main.rs`: in `service_deck_frame`, replace `frame_dur.as_secs_f64()` with `elapsed` in the smooth-advance line; add a cap of `frame_dur.as_secs_f64() * 2.0` so a stalled frame does not cause a large jump; update the comment to explain the reasoning
- [x] UPDATE Option C — `src/main.rs`: change the `large_drift` threshold multiplier from `0.1` to `0.3`; update the comment to explain the revised threshold is above typical accumulated drift but below a single beat at any practical BPM
- [x] UPDATE Option D — `src/main.rs`: in `service_deck_frame`, reduce the drift correction factor from `0.05` to `0.002`; update the comment to explain that with elapsed-advance removing steady-state lag, the correction is a noise amplifier rather than a genuine correction — the large_drift snap covers any genuine divergence

## Log

Extended experimentation during the build:

- Option E (f64 precision for display_samp through to render functions) was applied and live-toggled — no visible difference; removed.
- Spectrum animation and BPM flash were disabled — no difference; reverted.
- Drift correction block removed entirely — no visible difference over 0.002; reverted.
- 0.002 vs 0.0 toggled live — indistinguishable, confirming the drift correction is no longer the noise source at this factor. 0.002 retained as a safety net.

Remaining jitter not resolved. Two candidates for the planner: (1) background `BrailleBuffer` rebuilds changing `anchor_sample`/`samples_per_col` mid-frame; (2) `large_drift` snap firing during normal playback rather than only on seeks.

## Feedback

**Delivery status**: partially delivered

Option D (drift correction factor 0.05 → 0.002) gave a clear, visible improvement and is shipped at 0.9.16. Residual jitter remains after exhausting all position-advance and correction-factor experiments.

Findings that warrant planner attention:

- Disabling drift correction entirely (factor 0.0) was indistinguishable from 0.002 — the correction is no longer the noise source. The remaining jitter originates elsewhere in the render pipeline.
- Two uninvestigated candidates: (1) the background renderer thread rebuilding `BrailleBuffer` mid-frame, changing `anchor_sample` or `samples_per_col` while the render loop is computing `delta_half` — this would cause viewport jumps independent of `smooth_display_samp`; (2) `large_drift` snapping firing during normal playback rather than only after seeks — worth logging its frequency.
- Option E (f64 precision for display_samp) was inconclusive when live-toggled; the sub-sample truncation does not appear to be a significant contributor at current zoom levels.
