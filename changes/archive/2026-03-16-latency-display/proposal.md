# Proposal: Latency Display and Offset Compensation
**Status: Draft**

## 1 — Move latency from per-deck info bar to global bar

`audio_latency_ms` is a system-wide calibration value, not a per-deck property. It is currently rendered in every deck's info bar via `info_line_for_deck`. It should appear once, in the global bar.

### Change
- Remove `audio_latency_ms` from `info_line_for_deck` (parameter and render span).
- Add `lat:Xms` to the global bar, shown alongside the browser path when no notification is active. Only show when `> 0` (zero is the uncalibrated default and adds no information).

---

## 2 — Latency offset compensation

When latency changes by ±10ms, the handlers currently compensate `deck.tempo.offset_ms` in the opposite direction. The intent: a latency increase shifts the display window backward, which would visually move tick marks — so the offset is nudged forward to keep them in place.

This has two problems:
- Only deck 1 is compensated; deck 2 is left misaligned.
- It writes the compensated offset to cache, permanently coupling a calibration step to the track's beat grid.

### Options

**A — Remove compensation entirely**
Latency is a display offset; accept that changing it shifts the tick grid visually. Users adjusting latency are calibrating, not mixing — the visual shift is expected.

**B — Apply compensation to both decks**
Extend the handlers to also adjust `decks[1].tempo.offset_ms` by the same ±10ms amount. Keeps both grids visually stable.

### Recommendation

**Option A.** The offset compensation conflates two separate concerns (display calibration vs beat grid alignment). Users calibrating latency expect the display to shift — that is the point. Coupling it silently to `offset_ms` is surprising and writes unintended values to cache.

If beat grid alignment after latency change is needed, the user can adjust offset explicitly.
