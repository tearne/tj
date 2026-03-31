# Pitch Shift

## Intent
Add a per-deck pitch shift control that raises or lowers the key of a track in semitone steps, independent of playback speed, so that two tracks can be matched by key before mixing.

## Approach

**Library:** `soundtouch` crate (wraps SoundTouch C++ library, LGPL-2.1). Handles time-domain pitch shifting at constant tempo. Clamped to ±6 semitones.

**Audio chain:** Insert a `PitchSource<S>` wrapper between `FilterSource<TrackingSource>` and the mixer. When pitch is 0, `PitchSource` is a zero-cost passthrough. When non-zero, it feeds interleaved frames into a `SoundTouch` instance and drains the output into a `VecDeque<f32>` buffer, yielding one sample per `next()` call.

**Seeking:** `SeekHandle` gains a `flush_pitch: Arc<AtomicBool>`. Both `seek_to` and `seek_direct` set this flag. `PitchSource::next()` checks it; when set, calls `st.clear()`, drains the output buffer, and clears the flag. This prevents stale pitched samples surviving a seek.

**Pitch change:** When `pitch_semitones` changes between calls, `PitchSource` calls `st.set_pitch_semi_tones(new)` and flushes its output buffer to avoid a timbral smear from the previous setting.

**State in `Deck`:** `pitch_semitones: i8` (UI value, ±6 range, 0 = off). An `Arc<AtomicI8>` is shared with the audio thread via `AudioState`. No cache persistence.

**Display:** Pitch appears in the info line BPM bracket alongside the playback BPM, as a playback modification. When pitch is non-zero the bracket is shown even if BPM is not adjusted: `128.00 (+2st)`. When both are active: `128.00 (130.1  +2st)`. Not shown when pitch is 0.

**Key bindings (defaults):**
- Deck 1 pitch up: `5`, pitch down: `t`
- Deck 2 pitch up: `9`, pitch down: `o`

Both unused in the current keymap.

## Plan
- [x] ADD DEP — add `soundtouch` to `Cargo.toml`
- [x] ADD IMPL — `PitchSource<S>` in `src/audio/mod.rs`: struct, `Source` impl, passthrough when pitch = 0, flush on seek flag or pitch change
- [x] UPDATE IMPL — `SeekHandle`: add `flush_pitch: Arc<AtomicBool>`; set in `seek_to`, `seek_direct`, and `set_position`
- [x] UPDATE IMPL — `DeckAudio` and `Deck`: add `pitch_semitones: Arc<AtomicI8>`; wire `PitchSource` into chain in `build_deck`
- [x] UPDATE IMPL — `src/config/mod.rs`: add `Deck1PitchUp`, `Deck1PitchDown`, `Deck2PitchUp`, `Deck2PitchDown` actions and names
- [x] UPDATE IMPL — `resources/config.toml`: add default pitch key bindings (`5`/`t` deck 1, `9`/`o` deck 2)
- [x] UPDATE IMPL — `src/main.rs`: handle pitch actions, clamp to ±6
- [x] UPDATE IMPL — `src/render/mod.rs`: show `[+Nst]` / `[-Nst]` indicator in deck info bar

## Conclusion
Added per-deck pitch shift via the `soundtouch` crate. `PitchSource<S>` wraps `FilterSource<TrackingSource>` and is a zero-cost passthrough at 0 semitones. A `flush_pitch` flag on `SeekHandle` clears SoundTouch's buffer on seeks — but not on paused-nudge `set_position` calls, which make continuous tiny adjustments where flushing would restart SoundTouch's warmup and produce haywire audio. Keys `5`/`t` (deck 1) and `9`/`o` (deck 2) step pitch ±1 semitone, clamped to ±6. Pitch appears in the info line BPM bracket as a playback modification: `128.00 (+2st)` when only pitch is active, `128.00 (130.1  +2st)` when both are active. In the no-BPM / vinyl percentage branch it appears as a separate bracket after the percentage: `+0.0% (+2st)`.
