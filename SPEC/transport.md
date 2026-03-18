# Transport

## Deck Selection

- Two decks — Deck A and Deck B — operate independently. Each deck maintains its own track, transport, BPM, offset, volume, filter, metronome, zoom, waveform, tap accumulator, BPM analysis state, and notification state.
- One deck is active at a time. `g` selects Deck A; `h` selects Deck B. The active deck's control section is visually highlighted; the inactive deck's section is dim.
- All active-deck controls (transport, BPM, offset, metronome, nudge, tap, re-detect) apply to the active deck only. Zoom and detail height are global and apply to both decks simultaneously.
- Level and filter have dedicated per-deck bindings and may be adjusted on either deck regardless of which is active (see [keymap.md](keymap.md)).
- Global controls (quit, help, terminal refresh, file browser, deck select) are not deck-specific.
- At startup only Deck A is used. Deck B can be loaded by selecting it with `h` and opening the file browser.
- Audio latency is a single global setting shared across both decks.

## Playback

- Supports audio formats: FLAC, MP3, OGG Vorbis, WAV, AAC, OPUS.
- When playback reaches the end of the track, the transport pauses and the playhead returns to the start. The player view stays open and fully interactive.
- Decode runs on a background thread. A loading screen displays a progress bar showing decode progress.
- Decode completes and the deck is loaded paused. The user starts playback with `Space+Z`.
- Displays track metadata: title, artist, album, duration, current position. The track name (artist – title, or filename) is shown in the notification bar above the info bar.
- The TUI frame border title shows `tj vX.Y.Z` only.

## Beat Detection

- BPM is auto-detected from the audio on load, assuming a constant tempo throughout the track. Hash computation and BPM detection run on a background thread after decode; playback starts immediately with a 120 BPM placeholder.
- While BPM analysis is in progress, beat markers are suppressed, the beat indicator does not flash, and the BPM line shows an animated indicator (e.g. `BPM: --- [analysing ⠋]`). Beat jump uses the 120 BPM placeholder.
- When analysis completes and no BPM is yet established (fresh load, no cache, no tap or manual adjustment), the BPM is applied immediately. If a BPM is already established, the result is held as a pending confirmation.
- While a confirmation is pending, the notification bar shows a yellow prompt with the detected BPM and a countdown (e.g. `BPM detected: 124.40  [y] accept  [n] reject  (14s)`); the countdown number turns red when ≤ 5 s remain. The info bar right group is unaffected. Pressing `y` or `Enter` applies the result; any other key rejects it. After 15 seconds the result is auto-rejected and the pre-existing BPM and offset are preserved.
- `@` triggers a manual re-detection pass at any time. The result always goes through the confirmation step. Pressing `@` while analysis is in progress hides the spinner (the thread continues silently); pressing `@` again reconnects to the same thread rather than spawning a new one.
- The detected BPM is displayed to two decimal places.
- A beat phase offset (in milliseconds) can be adjusted at runtime to align the beat indicator with the audio. The offset and BPM are displayed in the UI.
- `offset_ms` is snapped to the nearest 10 ms boundary on load from the cache, ensuring `+`/`-` steps always land on multiples of 10 ms and 0 ms is always reachable. After each adjustment and on cache load, `offset_ms` is wrapped into `[0, beat_period_ms)` using `rem_euclid`, where `beat_period_ms` is derived from `base_bpm` rounded to the nearest 10 ms, ensuring the offset always remains on the 10 ms grid.
- The user can correct an inaccurate detection at runtime:
  - Per-deck BPM keys (`x`/`s` for Deck A, `v`/`f` for Deck B) increase/decrease the effective BPM by 0.01. Adjustments affect playback speed proportionally (relative to the detected BPM) and clamp to the range 40.0–240.0.
  - `b` tap-detects BPM: press in time with the beat. After 8 taps, `base_bpm` and `offset_ms` are set from the tap session. BPM is derived via linear regression of tap index against tap time (slope = beat period), which converges and stabilises as more taps are added. Taps with a residual exceeding half a beat period are treated as outliers and excluded before the final regression. Any active `f`/`v` speed ratio is preserved relative to the new `base_bpm`. The tap count is shown in the info bar (`tap:N`) while a session is active; tapping stops 2 seconds after the last tap.
  - Corrections are persisted to the cache immediately.
- Detected BPM and phase offset are cached in `~/.local/share/tj/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format. The cache also stores the last browser directory.
- Each cache entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- On quit, the current phase offset is persisted to the cache.

## Needle Drop

- A left mouse click anywhere on the Overview waveform seeks the transport to the start of the nearest bar marker at or to the left of the click position. Playback state is preserved — if playing, playback continues from the new position; if paused, the transport remains paused. The Detail view recentres on the new position immediately.

## Metronome

- `'` toggles metronome mode. While active, a click tone fires on every beat in sync with the current BPM and `offset_ms`. Only fires during playback; silent while paused. No click fires on the beat coinciding with activation; clicks begin from the following beat.
- The metronome fires based on the audio buffer write position (ahead of the speaker by `audio_latency_ms`), so the click arrives at the speaker on the beat when latency is correctly calibrated.
- The click tone reuses the latency calibration click sound.
- A `♪` (U+266A) symbol in red is shown in the info bar immediately after the BPM value while metronome is active.
- Metronome mode resets to off on each new track load.

## Nudge

- `c`/`d` nudge the transport backward/forward. Behaviour depends on the active nudge mode, toggled with `C`/`D`:
  - **`jump` mode** (default): each press (and key-repeat while held) seeks the playhead ±10ms.
  - **`warp` mode**: holding `c`/`d` applies a continuous ±10% speed offset; releasing returns to normal speed. While paused, drifts the transport position at ±10% of normal playback speed for as long as the key is held.
- The active nudge mode is shown in the info bar (`nudge:jump` / `nudge:warp`).
- While playing in warp mode, speed and pitch shift by ±10%; the audio output reflects the change within ~100ms.
- The nudge active state is indicated in the UI while a warp is held.
- While paused, each nudge step plays a short audio snippet at the new position — one half-column width of audio injected directly into the mixer. In jump mode a snippet fires on each key press/repeat; in warp mode snippets fire continuously at half-column intervals as the position drifts. Snippets play independently of the paused transport and do not interrupt each other.

## Beat Jump

- Eight dedicated beat jump actions cover four sizes (1, 4, 16, 64 beats) in each direction. Each action jumps by exactly N × `(60 / base_bpm)` audio seconds, which equals N beat periods at the effective playback BPM and lands precisely on the next tick mark.
- Jumping backward past the start clamps to position 0. Jumping forward past the end is a no-op.
- Seeking is implemented via an atomic position counter shared with the audio thread; the audio thread never pauses.
- A ~6ms fade-out before the cut and ~6ms fade-in after eliminate click artefacts without any perceptible gap.
