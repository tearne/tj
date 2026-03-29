# Deck

## Deck Selection

- Two decks — Deck 1 and Deck 2 — operate independently. Each deck maintains its own track, transport, BPM, offset, volume, filter, metronome, zoom, waveform, tap accumulator, BPM analysis state, and notification state.
- One deck is active at a time. `g` selects Deck 1; `h` selects Deck 2. The active deck's control section is visually highlighted; the inactive deck's section is dim.
- All active-deck controls (transport, BPM, offset, metronome, nudge, tap, re-detect) apply to the active deck only. Zoom and detail height are global and apply to both decks simultaneously.
- Level and filter have dedicated per-deck bindings and may be adjusted on either deck regardless of which is active (see [keymap.md](keymap.md)).
- Global controls (quit, help, vinyl mode toggle, terminal refresh, file browser, deck select) are not deck-specific.
- At startup only Deck 1 is used. Deck 2 can be loaded by selecting it with `h` and opening the file browser.
- Audio latency is a single global setting shared across both decks.

## Playback

- Supports audio formats: FLAC, MP3, OGG Vorbis, WAV, AAC, OPUS.
- When playback reaches the end of the track, the transport pauses and the playhead returns to the start. The player view stays open and fully interactive.
- Decode runs on a background thread. A loading screen displays a progress bar showing decode progress.
- Decode completes and the deck is loaded paused. The user starts playback with `Space+Z`.
- Displays track metadata: title, artist, album, duration, current position. The track name (artist – title, or filename) is shown in the notification bar above the info bar.
- The TUI frame border title shows `deck vX.Y.Z` only.

## Beat Detection

- BPM is auto-detected from the audio on load, assuming a constant tempo throughout the track. Hash computation and BPM detection run on a background thread after decode; playback starts immediately with a 120 BPM placeholder.
- While BPM analysis is in progress, beat markers are suppressed, the beat indicator does not flash, and the BPM line shows an animated indicator (e.g. `BPM: --- [analysing ⠋]`). Beat jump uses the 120 BPM placeholder.
- When analysis completes and no BPM is yet established (fresh load, no cache, no tap or manual adjustment), the BPM is applied immediately. If a BPM is already established, the result is held as a pending confirmation.
- While a confirmation is pending, the notification bar shows a yellow prompt with the detected BPM and a countdown (e.g. `BPM detected: 124.40  [y] accept  [n] reject  (14s)`); the countdown number turns red when ≤ 5 s remain. The info bar right group is unaffected. Pressing `y` or `Enter` applies the result; any other key rejects it. After 15 seconds the result is auto-rejected and the pre-existing BPM and offset are preserved.
- `@` triggers a manual re-detection pass at any time. The result always goes through the confirmation step. Pressing `@` while analysis is in progress hides the spinner (the thread continues silently); pressing `@` again reconnects to the same thread rather than spawning a new one.
- The native BPM (`base_bpm`) is displayed to two decimal places, matching the 0.01 resolution of the `X`/`S` adjustment keys. The playback BPM (after `x`/`s` adjustment) is displayed to one decimal place, matching the 0.1 resolution of those keys. All underlying values retain full precision; rounding is display-only.
- A beat phase offset (in milliseconds) can be adjusted at runtime to align the beat indicator with the audio. The offset and BPM are displayed in the UI.
- `offset_ms` is snapped to the nearest 10 ms boundary on load from the cache, ensuring `+`/`-` steps always land on multiples of 10 ms and 0 ms is always reachable. After each adjustment and on cache load, `offset_ms` is wrapped into `[0, beat_period_ms)` using `rem_euclid`, where `beat_period_ms` is derived from `base_bpm` rounded to the nearest 10 ms, ensuring the offset always remains on the 10 ms grid.
- The user can correct an inaccurate detection at runtime:
  - Per-deck BPM keys (`x`/`s` for Deck 1, `v`/`f` for Deck 2) increase/decrease the effective BPM by 0.1. Adjustments affect playback speed proportionally (relative to the detected BPM) and clamp to the range 40.0–240.0.
  - `b` tap-detects BPM: press in time with the beat. After 8 taps, `base_bpm` and `offset_ms` are set from the tap session. BPM is derived via linear regression of tap index against tap time (slope = beat period), which converges and stabilises as more taps are added. Taps with a residual exceeding half a beat period are treated as outliers and excluded before the final regression. Any active `f`/`v` speed ratio is preserved relative to the new `base_bpm`. The tap count is shown in the info bar (`tap:N`) while a session is active; tapping stops 2 seconds after the last tap.
  - Corrections are persisted to the cache immediately.
- Detected BPM and phase offset are cached in `~/.config/deck/cache.json`, keyed by a Blake3 hash of the decoded audio samples. This makes the cache invariant of filename, tags, and container format. The cache also stores the last browser directory.
- Each cache entry includes the filename at time of first detection as a human-readable hint to aid manual cache management.
- On quit, the current phase offset is persisted to the cache.

## Cue Point

- Each deck has a single cue point stored as a sample position. It is persisted to cache alongside BPM and offset.
- **Cue set** (`A` / `D`): when the deck is paused, sets the cue at the current position and snaps the beat grid so that a tick falls on the cue (`offset_ms` is recalculated). Does nothing while playing.
- **Cue play** (`Space+A` / `Space+D`): jumps to the cue position and maintains the current play state — if playing, playback continues from the cue; if paused, the transport stays paused at the cue. Does nothing if no cue is set.
- The cue position acts as the zero datum for the beat grid: whenever `base_bpm` changes (manual adjustment or re-detection), `offset_ms` is recalculated to keep a tick on the cue position.
- BPM tap does not disturb the cue point; the tapped grid lands where it lands.
- The cue column is shown as a green marker in both the overview and detail waveforms.

## Gain Trim

- Each deck has an independent gain trim applied to the audio signal after the filter and before the fader. The trim range is ±12 dB in 1 dB steps.
- `J` / `M` (Deck 1) and `K` / `<` (Deck 2) increase / decrease gain by 1 dB. Clamps silently at ±12 dB.
- Gain is applied as a linear multiplier (`10^(dB/20)`) in the audio signal chain, after the filter and before PFL routing.
- Gain is persisted to the cache alongside BPM and offset, and restored when the track is loaded.
- The detail info bar shows a single character gain indicator immediately after the level closing bracket. It uses `▁▂▃▄▅▆▇` to represent the range −12 dB to +12 dB, with `▄` at 0 dB. The indicator is grey at 0 dB and dim amber at any non-zero value.

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
- In vinyl mode, beat jump keys are remapped to fixed time intervals (see *Vinyl Mode* below).
- Jumping backward past the start clamps to position 0. Jumping forward past the end is a no-op.
- Seeking is implemented via an atomic position counter shared with the audio thread; the audio thread never pauses.
- A ~6ms fade-out before the cut and ~6ms fade-in after eliminate click artefacts without any perceptible gap.

## Vinyl Mode

Vinyl mode is a global toggle (`` ` ``) that applies to both decks simultaneously. It hides the BPM machinery and presents a cleaner interface for ear-based pitch matching.

**Speed control** — `vinyl_speed: f32` (per deck, 1.0 = nominal) is the authoritative playback speed in vinyl mode, passed directly to `player.set_speed`. The BPM keys (`x`/`s`, `v`/`f`) adjust `vinyl_speed` in 0.001 steps (±0.1%); this is the only change: the same keys, the same step feel, a different unit. `vinyl_speed` resets to 1.0 whenever a new track is loaded in vinyl mode.

**Speed display** — The BPM field in the info bar is replaced by a percentage (e.g. `+0.3%`, `0.0%`) derived from `vinyl_speed`. Beat flash, phase offset, and metronome indicator are hidden.

**Waveform** — Beat tick marks and the cue column are hidden. The waveform itself is unchanged.

**Beat jumps** — Remapped to fixed time intervals equal to N beats × 0.5 s (the beat period at 120 BPM):

| Keys | Beat mode | Vinyl mode |
|------|-----------|------------|
| `1` / `q` (Deck 1), `3` / `e` (Deck 2) | ±4 bars (16 beats) | ±8 s |
| `2` / `w` (Deck 1), `4` / `r` (Deck 2) | ±8 bars (32 beats) | ±16 s |
| `Space+1` / `Space+q` (Deck 1), `Space+3` / `Space+e` (Deck 2) | ±1 beat | ±0.5 s |
| `Space+2` / `Space+w` (Deck 1), `Space+4` / `Space+r` (Deck 2) | ±4 beats | ±2 s |

**BPM analysis** — Does not run while vinyl mode is active. The redetect key has no effect in vinyl mode.

**Mode transition: vinyl → beat** — On switching back to beat mode, `vinyl_speed` is converted to a BPM adjustment: `bpm = base_bpm × vinyl_speed`. The player speed is unchanged. The beat grid and display immediately reflect the new BPM.

**Mode transition: beat → vinyl** — `vinyl_speed` is set to the current `bpm / base_bpm` so audio speed does not change.

**Session persistence** — The active mode is stored in the cache file. On startup, the last-used mode is restored; the default for a fresh installation is beat mode.
