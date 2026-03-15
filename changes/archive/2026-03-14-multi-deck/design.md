# Design: Multi-Deck
**Status: Approved**

## Approach

### Core structural change: `Deck` struct

Almost all of `tui_loop`'s local state is per-deck. It gets extracted into a `Deck` struct. The remaining globals (keymap, display config, terminal, cache, browser dir, mixer, frame timing, help popup, space-held flag) stay flat in the loop.

```rust
struct DeckAudio {
    player: Player,
    seek_handle: SeekHandle,
    mono: Arc<Vec<f32>>,
    waveform: Arc<WaveformData>,
    sample_rate: u32,
    filter_offset_shared: Arc<AtomicI32>,
}

struct TempoState {
    bpm: f32,
    base_bpm: f32,
    offset_ms: i64,
    bpm_rx: mpsc::Receiver<(String, f32, i64, bool)>,
    analysis_hash: Option<String>,
    bpm_established: bool,
    pending_bpm: Option<(String, f32, i64, Instant)>,
    redetecting: bool,
    redetect_saved_hash: Option<String>,
    background_rx: Option<mpsc::Receiver<(String, f32, i64, bool)>>,
}

struct TapState {
    tap_times: Vec<f64>,
    last_tap_wall: Option<Instant>,
    was_tap_active: bool,
}

struct DisplayState {
    smooth_display_samp: f64,
    last_scrub_samp: f64,
    last_viewport_start: usize,
    overview_rect: ratatui::layout::Rect,
    last_bar_cols: Vec<usize>,
    last_bar_times: Vec<f64>,
    zoom_idx: usize,
    detail_height: usize,
    palette_idx: usize,
}

struct SpectrumState {
    chars: [char; 16],
    bg: [bool; 16],
    bg_accum: [bool; 16],
    last_update: Option<Instant>,
    last_bg_update: Option<Instant>,
}

struct BrailleRenderer {
    cols: Arc<AtomicUsize>,
    rows: Arc<AtomicUsize>,
    zoom_at: Arc<AtomicUsize>,
    style: Arc<AtomicUsize>,
    shared: Arc<Mutex<Arc<BrailleBuffer>>>,
    display_pos: Arc<AtomicUsize>,
    _stop_guard: StopOnDrop,
}

struct Deck {
    filename: String,
    track_name: String,
    total_duration: Duration,
    volume: f32,
    filter_offset: i32,
    nudge: i8,
    nudge_mode: NudgeMode,
    metronome_mode: bool,
    last_metro_beat: Option<i128>,
    active_notification: Option<Notification>,

    audio: DeckAudio,
    tempo: TempoState,
    tap: TapState,
    display: DisplayState,
    spectrum: SpectrumState,
    renderer: BrailleRenderer,
}
```

A factory function `Deck::new(audio, bpm_rx, audio_latency_ms, config_notice)` initialises a deck and spawns its background braille thread.

### Track loading moves inside `tui_loop`

Currently `main()` loops: decode → setup audio → call `tui_loop` → repeat. With two decks that model breaks — one deck can reload while the other plays.

`tui_loop` is restructured to own the full lifecycle:
- The decode loop (currently in `main()`) moves inside, behind a `DeckLoad` helper that drives decode on a background thread and shows a per-deck loading indicator.
- `main()` becomes much simpler: terminal setup, cache load, browser (if needed), initial path, enter `tui_loop` once. `tui_loop` no longer returns a next path; it only returns on quit.
- The file browser, when returning a selection, calls `load_deck(active, path, &mixer, &mut cache)` which tears down the old deck's audio and spawns a new one.

### Deck B starts empty

At startup only Deck A is loaded. Deck B exists as `Option<Deck>`, initially `None`. The user selects Deck B with `h` and opens the file browser to load a track. Until loaded, Deck B's panels show a placeholder.

### Input routing

- `g` → `active_deck = 0`; `h` → `active_deck = 1`
- All active-deck actions dispatch to `decks[active_deck]` (or are a no-op if Deck B is not yet loaded)
- **Fixed per-deck level/filter actions** (not routed through active deck):
  - New actions: `DeckALevelUp`, `DeckALevelDown`, `DeckAFilterIncrease`, `DeckAFilterDecrease`
  - New actions: `DeckBLevelUp`, `DeckBLevelDown`, `DeckBFilterIncrease`, `DeckBFilterDecrease`
  - Default bindings: `j`/`m` → Deck A level, `u`/`7` → Deck A filter; `k`/`,` → Deck B level, `i`/`8` → Deck B filter
  - These are intercepted before the active-deck dispatch and applied directly to the relevant deck

### Layout

Five vertical sections; active deck's control section gets a highlight border or indicator:

```
┌─ tj v0.5.0 ───────────────────────────────────────────────┐
│  Detail waveform — Deck A          (Constraint::Min(4))    │
│  Detail waveform — Deck B          (Constraint::Min(4))    │
│  Notification bar A                (Constraint::Length(1)) │
│  Info bar A                        (Constraint::Length(1)) │
│  Overview A                        (Constraint::Length(4)) │
│  Notification bar B                (Constraint::Length(1)) │
│  Info bar B                        (Constraint::Length(1)) │
│  Overview B                        (Constraint::Length(4)) │
│  Global status bar                 (Constraint::Length(1)) │
└────────────────────────────────────────────────────────────┘
```

The active deck's control section (notification + info + overview) has its label or border rendered in the palette treble colour; the inactive deck is dim.

### Global status bar

A single row pinned to the bottom of the inner area. Content priority:

1. **System notification** — transient messages not tied to either deck (e.g. config parse warning). Shown until expired; same `Notification` struct with an expiry `Instant`.
2. **Idle status line** — shown when no system notification is active. Left-to-right, space-permitting:
   - Current browser working directory (already available as `browser_dir`)
   - *(deferred)* Global output level / clip indicator — requires mixer peak monitoring
   - *(deferred)* Playlist name / position — requires playlist feature
   - *(deferred)* CPU / RAM — requires background OS polling

The `tui_loop` gains a `global_notification: Option<Notification>` local. The existing config-parse notice (currently routed to Deck A) is redirected here. The idle content is rendered dim.

### Per-deck braille threads

Each `Deck` owns its own background braille render thread (as now) plus a `StopOnDrop` guard. The thread is spawned in `Deck::new` and stopped automatically when the deck is dropped or reloaded. Both threads run concurrently; the UI reads from whichever buffer it needs each frame.

### Config / keymap

New default bindings added to the embedded config. Existing `LevelUp`/`LevelDown`/`FilterIncrease`/`FilterDecrease` actions become the Deck A fixed variants. `DeckBLevel*` and `DeckBFilter*` are new. `DeckSelect` remains a configurable binding.

---

## Tasks

1. ✓ **Impl**: Define `Deck` struct and `Deck::new()`; extract single-deck from current `tui_loop` locals; verify single-deck behaviour unchanged
2. ✓ **Impl**: Move track loading into `tui_loop`; simplify `main()`; internal `load_deck()` replaces the outer loop
3. ✓ **Impl**: Add Deck B (`Option<Deck>`); deck select (`g`/`h`); route all active-deck actions through `decks[active_deck]`; browser loads into active deck
4. ✓ **Impl**: Fixed per-deck level/filter actions; new `DeckB*` actions; update keymap defaults; intercept before active-deck dispatch
5. ✓ **Impl**: Restructure layout into five sections; render both decks' control sections; active deck highlight; Deck B placeholder when empty; global status bar (system notifications + idle CWD display)
6. ✓ **Verify**: Build clean; manual test — load deck A, load deck B, independent transport/level/filter, deck switching
7. ✓ **Process**: Confirm ready to archive
