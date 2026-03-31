# No-BPM Beat Mode

## Intent

In beat mode with no BPM established, the display behaves inconsistently: the speed percentage flashes yellow as if on a beat, and pressing BPM-adjust keys (`s`/`x`) causes the display to jump to the default 120 BPM before stepping. A track with no BPM in beat mode should behave identically to a track in vinyl mode — percentage-based speed display with no beat flash, and speed adjustment working directly on the playback rate rather than through BPM arithmetic.

## Approach

Widen existing conditions to treat `!bpm_established` in beat mode the same as vinyl mode. No new code paths.

1. **`src/render/mod.rs` — `info_line_for_deck`**: `beat_active` gains `&& deck.tempo.bpm_established`; `pct` condition widens from `vinyl_mode` to `vinyl_mode || !deck.tempo.bpm_established`; offset/metronome guard widens from `!vinyl_mode` to `!vinyl_mode && deck.tempo.bpm_established`.
2. **`src/main.rs` — `BpmIncrease`/`BpmDecrease` ×4**: condition widens from `vinyl_mode` to `vinyl_mode || !d.tempo.bpm_established`.

## Plan
- [x] UPDATE IMPL — `src/render/mod.rs`: `beat_active`, `pct`, offset/metronome visibility
- [x] UPDATE IMPL — `src/main.rs`: `BpmIncrease`/`BpmDecrease` ×4
- [-] UPDATE IMPL — `src/main.rs`: `BaseBpmIncrease`/`BaseBpmDecrease` — no-op when `!bpm_established` (reverted: these keys are the manual BPM entry path)

## Conclusion
In `info_line_for_deck`, `beat_active` now requires `bpm_established` — no yellow flash before BPM is known. The `pct` calculation uses `vinyl_speed` when `!bpm_established`, matching the control path. The offset/metronome guard was narrowed to hide both when no BPM is established. In the four `BpmIncrease`/`BpmDecrease` handlers, the vinyl-speed path now triggers on `vinyl_mode || !bpm_established`, so speed adjustments on unanalysed tracks work directly on `vinyl_speed` without materialising a phantom 120 BPM. `BaseBpmIncrease`/`BaseBpmDecrease` (Shift+S/X) is left unchanged — it remains the manual BPM entry path and correctly sets `bpm_established` when invoked.
