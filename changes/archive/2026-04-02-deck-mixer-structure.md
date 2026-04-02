# Deck / Mixer Structure

## Intent
The keyboard-layout change established two distinct conceptual domains: **deck controls** (operating on the selected deck — play/pause, pitch, nudge, BPM, cue, metronome) and the **mixer** (addressing each deck directly — level, gain, filter, PFL). Currently the `Deck` struct conflates both domains, and `SPEC/deck.md` covers both without distinguishing them. This change asks whether those domains warrant independent structural expression — a `Mixer` type in the code and a `SPEC/mixer.md` in the specification — and, if so, carries out the reorganisation.

## Approach

The `Deck` struct in `src/deck/mod.rs` currently holds five mixer fields inline — `volume`, `gain_db`, `pfl_level`, `filter_offset`, `filter_poles` — mixed with deck-control state. These are extracted into a `Mixer` struct defined in the same file, following the same pattern as `TempoState`, `TapState`, and `DisplayState`. `Deck` then holds `mixer: Mixer`. All call sites in `main.rs` are updated (`d.volume` → `d.mixer.volume`, etc.). The atomic mirror fields in `DeckAudio` (`deck_volume_atomic`, `gain_linear`, `pfl_level`, `filter_offset_shared`, `filter_state_reset`, `filter_poles`) stay in `DeckAudio` — they are the audio-thread interface, not domain state. `pfl_active_deck` (the cross-deck routing lock) stays in `main.rs`; relocating it would require broader structural changes beyond the scope of this review.

A new `SPEC/mixer.md` is created. The Gain Trim and PFL Monitor sections are moved from `SPEC/deck.md` to it. A Filter section is added to `SPEC/mixer.md` — filter behaviour is currently documented only in `config.md` key bindings; it deserves a proper spec section alongside the other mixer controls. `SPEC/deck.md` is updated to remove the moved sections and adjust any cross-references.

No new `src/mixer/` module is introduced — the mixer logic does not yet have the density or independence to justify a separate module. This can be revisited if mixer behaviour grows.

Review cadence: at the end.

## Plan

- [x] ADD SPEC — Create `SPEC/mixer.md`: move Gain Trim and PFL Monitor sections from `SPEC/deck.md`; add a new Filter section covering filter sweep range, reset, slope toggle, and display behaviour
- [x] UPDATE SPEC — `SPEC/deck.md`: remove moved sections; update Deck Selection paragraph to reference mixer as a separate concern; adjust any cross-references
- [x] UPDATE IMPL — Extract `Mixer` struct from `Deck` in `src/deck/mod.rs`: fields `volume`, `gain_db`, `pfl_level`, `filter_offset`, `filter_poles`; replace inline fields on `Deck` with `mixer: Mixer`; update `Deck::new` initialisation
- [x] UPDATE IMPL — Update all `d.volume`, `d.gain_db`, `d.pfl_level`, `d.filter_offset`, `d.filter_poles` call sites in `main.rs` to `d.mixer.*`
- [x] UPDATE IMPL — Update `cache_entry_for_deck` in `src/deck/mod.rs` to read from `d.mixer` where it references the moved fields

## Conclusion

`Mixer` struct extracted with fields `volume`, `gain_db`, `pfl_level`, `filter_offset`, `filter_poles`. All call sites in `main.rs`, `render/mod.rs`, and `deck/mod.rs` updated to `d.mixer.*`. `SPEC/mixer.md` created with Level, Gain Trim, Filter, and PFL Monitor sections; Gain Trim and PFL Monitor removed from `SPEC/deck.md`. The automated field rename missed `deck.` and `od.` prefixes and the `CacheEntry.gain_db` field — corrected with targeted fixes. Compiles clean.
