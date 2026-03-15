# Design: Shared Detail Waveform Pipeline
**Status: Draft**

## Approach

### Root cause

Frame timing (`poll_dur`, `elapsed`, `col_secs`) is derived from the active deck's `zoom_idx` and renderer. The inactive deck's `smooth_display_samp` is therefore advanced at the active deck's rate, with the active deck's cap (`col_secs * 4.0`). If zoom levels differ the inactive deck's viewport slides through its buffer at the wrong pace. Even at matching zoom levels, two independent background threads compute on independent schedules, so the UI can read buffers at mismatched positions within the same frame.

### Shared zoom and height

`zoom_idx` and `detail_height` move from `DisplayState` (per-deck) into `tui_loop` as plain `usize` locals. `{`/`}` and `-`/`=` act on these globals. `deck.renderer.zoom_at.store(...)` is replaced by a single store to the shared renderer. All reads of per-deck zoom/height are updated to use the globals.

### `SharedDetailRenderer`

Replaces the two per-deck `BrailleRenderer` instances. One background thread produces two `BrailleBuffer`s in the same pass, at the same `col_samp`, from the same `cols`/`rows`/`zoom`/`style` snapshot.

```rust
struct SharedDetailRenderer {
    cols:          Arc<AtomicUsize>,
    rows:          Arc<AtomicUsize>,
    zoom_at:       Arc<AtomicUsize>,
    style:         Arc<AtomicUsize>,
    waveform_a:    Arc<Mutex<Option<Arc<WaveformData>>>>,
    waveform_b:    Arc<Mutex<Option<Arc<WaveformData>>>>,
    display_pos_a: Arc<AtomicUsize>,   // interleaved sample index (× channels)
    display_pos_b: Arc<AtomicUsize>,
    channels_a:    Arc<AtomicUsize>,
    channels_b:    Arc<AtomicUsize>,
    sample_rate_a: Arc<AtomicUsize>,
    sample_rate_b: Arc<AtomicUsize>,
    shared_a:      Arc<Mutex<Arc<BrailleBuffer>>>,
    shared_b:      Arc<Mutex<Arc<BrailleBuffer>>>,
    _stop_guard:   StopOnDrop,
}
```

The background thread:
1. Reads `cols`, `rows`, `zoom`, `style`, `col_samp` (same for both decks).
2. Reads `display_pos_a` / `display_pos_b` and checks drift against each deck's last anchor. Recomputes if **either** deck has drifted ≥ ¾ cols, or if any layout/zoom/style parameter changed.
3. Locks `waveform_a` / `waveform_b`. If a slot is `None`, produces an all-zero-peak buffer for that deck.
4. Computes peaks for both decks independently (each anchored to its own position), then writes both `Arc<BrailleBuffer>`s under their respective locks.

Both buffers therefore share identical `samples_per_col` and `col_samp`, so their column grids are byte-for-byte compatible wherever they overlap — the same guarantee that exists today for a single deck across buffer handoffs.

On deck load, the relevant `waveform_*` slot is updated in place (no thread recreation). On deck unload, the slot is set to `None`.

`Deck` loses its `renderer: BrailleRenderer` field entirely.

### Unified frame timing

A single `col_secs` is computed from the shared `zoom_idx` and the shared renderer's `cols`. `elapsed` and `poll_dur` are derived from this, then used to advance **both** decks' `smooth_display_samp` in the same frame. The inactive deck's position update moves from the loop-top service block into the main per-frame timing path alongside the active deck.

### `render_detail_waveform` signature

Currently reads `deck.renderer.shared` internally. With the shared renderer the buffer is resolved outside the draw closure and passed in:

```rust
fn render_detail_waveform(
    frame: &mut Frame, buf: &Arc<BrailleBuffer>,
    display: &mut DisplayState, area: Rect,
    display_cfg: &DisplayConfig, display_pos_samp: usize,
)
```

`render_detail_waveform_inactive` gets the same treatment (passing `&Arc<BrailleBuffer>` rather than the full deck).

### Layout

A shared detail info bar (`Constraint::Length(1)`) is inserted as the first row of the detail section, above both waveforms. It shows the zoom level (e.g. `zoom: 4s`) in dim style. The zoom field is removed from each deck's info bar right group.

Updated layout (10 rows):
```
┌─ tj ──────────────────────────────────────────────────────┐
│  Detail info bar         (Constraint::Length(1))           │
│  Detail waveform — Deck A  (Constraint::Length(height))    │
│  Detail waveform — Deck B  (Constraint::Length(height))    │
│  Notification bar A        (Constraint::Length(1))         │
│  Info bar A                (Constraint::Length(1))         │
│  Overview A                (Constraint::Length(4))         │
│  Notification bar B        (Constraint::Length(1))         │
│  Info bar B                (Constraint::Length(1))         │
│  Overview B                (Constraint::Length(4))         │
│  Global status bar         (Constraint::Length(1))         │
└────────────────────────────────────────────────────────────┘
```

## Tasks

1. ✓ **Impl**: Lift `zoom_idx` and `detail_height` out of `DisplayState` into `tui_loop` globals; update `-`/`=` and `{`/`}` handlers and all reads/stores
2. ✓ **Impl**: Define `SharedDetailRenderer`; single background thread producing two buffers; on-load waveform slot update; `None` slot → zero-peak buffer
3. ✓ **Impl**: Remove `renderer` from `Deck`; wire `display_pos_a`/`b` stores into the per-frame path; pass resolved buffers into `render_detail_waveform` / `render_detail_waveform_inactive`
4. ✓ **Impl**: Unify frame timing — single `col_secs`/`elapsed`/`poll_dur` from shared zoom; both smooth positions advanced in the same frame
5. ✓ **Impl**: Layout — insert shared detail info bar; render zoom level; remove zoom from per-deck info bar right group
6. **Verify**: Build clean; manual test — both waveforms stable when switching active deck; zoom/height apply to both simultaneously; detail info bar renders correctly
7. **Process**: Confirm ready to archive
