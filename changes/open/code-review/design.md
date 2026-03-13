# Design: Code Review
**Status: Approved**

## Approach

The codebase is a single `src/main.rs` of ~2860 lines. All findings are documented here. Tasks are split into two categories:

- **Immediate**: clear wins — dead code, performance, small deduplication. Applied directly.
- **Structural**: larger concerns involving module boundaries and function decomposition. These are documented as findings and will feed into the multi-deck design rather than being applied ad-hoc.

---

## Findings

### Performance

**P1 — `hash_mono` iterates sample-by-sample** (`2464–2470`)
Calls `hasher.update(&s.to_le_bytes())` in a loop over every mono sample (~13M iterations for a 5-min track). The `blake3::Hasher` accepts bulk byte slices. The `&[f32]` should be reinterpreted as `&[u8]` and hashed in one call via `bytemuck::cast_slice` or a manual pointer cast.

**P2 — Hann window recomputed every spectrum update** (`1865–1867`)
`compute_spectrum` allocates and computes 4096 cosine values on every call (~twice per beat). The window is a pure function of `N` — it should be a `static` (e.g. via `std::sync::OnceLock`).

**P3 — Spectrum filter buffer allocated every call** (`1870–1884`)
When `filter_offset != 0`, allocates a `Vec<f32>` of 4096 elements on each spectrum update. Could use a fixed-size array on the stack (`[f32; N]`), or reuse a buffer. When `filter_offset == 0` an empty `Vec::new()` is created and immediately discarded — minor but avoids a branch.

**P4 — `bar_cols.contains(&c)` is O(n_bars) per column** (`872`)
Called in the inner loop of the overview render (once per column, per frame). For dense bar grids this is O(cols × bars). A `HashSet<usize>` or a sorted binary search would be O(1)/O(log n). Low priority at current zoom levels but worth noting for correctness of intent.

**P5 — `entries_snapshot` clones the full cache HashMap** (`2549–2551`)
Called once per track load to pass cache data to the BPM analysis thread. For large caches (many tracks) this is a noticeable allocation. The thread only reads the result for the current hash; sending the full map is wasteful. However, since the hash is not yet known at snapshot time, this requires a small API change to the BPM thread. Low priority but worth tracking.

---

### Dead Code

**D1 — `seek_micro_fade` is never called** (`2329–2338`)
The method is fully implemented, has a doc comment, and references `total_frames` which is immediately silenced with `let _ = total_frames`. This is dead code — remove.

**D2 — Empty `// Beat marker helpers` section** (`1537–1541`)
The section header exists but contains no code. The beat-line logic was moved inline into `tui_loop`. Remove the header.

**D3 — Orphaned doc comment for `shift_braille_half`** (`1572`)
The comment "Combine two adjacent braille bytes into a half-column-shifted result." appears directly before the `NudgeMode` enum definition — it was left behind when `shift_braille_half` was moved to line 1945. Remove it from its current position (the function already has its own doc comment at 1943).

**D4 — `total_frames` suppression in `seek_micro_fade`** (`2337`)
`let _ = total_frames;` is a symptom of D1. Removed together with the function.

---

### Duplication

**U1 — Terminal teardown sequence repeated 6× in `main`** (`108, 112–113, 123–124, 177–178, 188–189, 265–266, 273–274`)
The exact two-line sequence appears at every early exit point. Extract to `fn cleanup_terminal()`.

**U2 — Quiet-frame search duplicated in `seek_to` and `seek_direct`** (`2308–2315`, `2352–2359`)
Both methods implement an identical search for the lowest-amplitude frame within ±10ms of the target. Extract to `SeekHandle::find_quiet_frame(target_secs) -> usize`.

**U3 — Two `HOME` implementations** (`1772`, `2472–2476`)
`home_dir()` uses `var_os`; `cache_path()` re-implements the same logic using `var`. `cache_path` should call `home_dir()`.

---

### Encapsulation / Structural (for multi-deck design input)

These are not immediate tasks — they are documented here to inform the multi-deck proposal.

**S1 — `tui_loop` is a ~1260-line monolith** (`277–1535`)
Violates P1 and P2. It contains: the detail braille background thread spawn, per-frame state updates (smooth position, beat flash, calibration pulse, metronome, spectrum), the full draw closure (~500 lines), adaptive frame rate logic, the event loop, and all action dispatch. A reader cannot understand the high-level flow without reading the entire function.

Natural decomposition for multi-deck:
- `PlayerState` struct — `bpm`, `base_bpm`, `offset_ms`, `volume`, `nudge`, `nudge_mode`, `filter_offset`, `calibration_mode`, `audio_latency_ms`, `metronome_mode`
- `DisplayState` struct — `spectrum_chars`, `spectrum_bg`, `spectrum_bg_accum`, `last_*` timers, `zoom_idx`, `detail_height`, `palette_idx`, `smooth_display_samp`
- `fn update_display_position(...)` — smooth position + latency compensation
- `fn update_spectrum(...)` — spectrum timing + compute
- `fn draw_player(...)` — the draw closure, extracted
- `fn handle_action(...)` — action dispatch

**S2 — Module boundaries for multi-deck**
The natural modules, none of which exist yet:
- `audio`: `TrackingSource`, `FilterSource`, `SeekHandle`, `decode_audio`, `scrub_audio`, `play_click_tone`
- `player`: `PlayerState`, BPM logic, tap BPM
- `waveform`: `WaveformData`, `BrailleBuffer`, `render_braille`, `shift_braille_half`, `bar_tick_cols`, `compute_spectrum`
- `browser`: `BrowserState`, `BrowserEntry`, `EntryKind`, `BrowserResult`, `run_browser`, `is_audio`
- `cache`: `Cache`, `CacheEntry`, `CacheFile`, `hash_mono`, `detect_bpm`
- `config`: `load_config`, `resolve_config`, `parse_keymap`, `parse_display_config`, `DisplayConfig`, `Action`, `KeyBinding`, `ACTION_NAMES`

**S3 — `SeekHandle` owns Arc references duplicated from `TrackingSource`**
Both structs hold `Arc` clones of `position`, `fade_remaining`, `fade_len`, `pending_target`, `samples`. This is intentional (cross-thread sharing) but creates 10 `Arc::clone` calls in `main`. For multi-deck, each deck would need its own set. Wrapping them in a single `Arc<TrackState>` struct would reduce the fan-out.

**S4 — `compute_spectrum` couples display and audio**
It takes `filter_offset: i32` and re-runs the biquad to match `FilterSource`'s output. This is correct but creates two codepaths for the same filter. In a future where the filtered audio is accessible from the audio thread, the spectrum could read from that buffer instead.

---

## Tasks

1. **Impl**: Fix D1, D2, D3, D4 — remove dead code
2. **Impl**: Fix U1 — extract `cleanup_terminal()`
3. **Impl**: Fix U2 — extract `SeekHandle::find_quiet_frame()`
4. **Impl**: Fix U3 — `cache_path` uses `home_dir()`
5. **Impl**: Fix P1 — `hash_mono` bulk hashing
6. **Impl**: Fix P2 — Hann window as `OnceLock` static
7. **Process**: Archive code review; draft `multi-deck` proposal incorporating structural findings S1–S4 as design constraints
