# Audio

## Level Control

- Each deck has an independent playback level, adjustable in 5% steps from 0% to 100%. Deck A uses `j` (up) / `m` (down); Deck B uses `k` (up) / `,` (down). These bindings are active regardless of which deck is selected. The current level is displayed in the info bar as `level:N%`. Changes take effect immediately without interrupting playback. Level is not persisted between sessions.

## HPF / LPF Filter

- A single `filter_offset` parameter (range −16 to +16, default 0) controls a real-time second-order Butterworth IIR filter on the playback output:
  - `0` — flat (filter bypassed).
  - `−1` to `−16` — low-pass filter; more negative = lower cutoff frequency.
  - `+1` to `+16` — high-pass filter; more positive = higher cutoff frequency.
- Deck A: `u` decreases `filter_offset` by 1; `7` increases it by 1. `Space+u` or `Space+7` snaps to flat.
- Deck B: `i` decreases `filter_offset` by 1; `8` increases it by 1. These bindings are active regardless of which deck is selected.
- Cutoff frequencies are logarithmically spaced from ~40 Hz to ~18 kHz across the ±1–±16 range. Each step corresponds to exactly one character of the spectrum strip.
- Filter state is visible in the spectrum strip (grey shading on attenuated bins) and not shown as separate text.
- The spectrum analyser reflects the filtered output.
- Filter state is not persisted between sessions; it always initialises to flat.

## Audio Latency Calibration

- An `audio_latency_ms` value shifts all visual rendering backward by a fixed number of milliseconds, compensating for audio output latency. The effective display position is `smooth_display_samp − audio_latency_ms × sample_rate / 1000`. This affects the waveform viewport, beat markers, beat flash, and overview playhead.
- `[` / `]` adjust `audio_latency_ms` in 10ms steps (clamped 0–250ms) at any time. Each adjustment simultaneously compensates `offset_ms` by the opposite amount (then wrapped), keeping tick markers anchored to their heard position while the waveform display shifts. The recommended workflow: tap BPM with `b` until ticks are locked to the heard beat, then nudge `[`/`]` until ticks align with the waveform peaks.
- `audio_latency_ms` is stored as a single global value in the cache (alongside per-track entries). It is loaded on startup and saved on each change and on quit.
