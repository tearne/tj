# Design: Initial Implementation
**Status: Draft**

## Approach

### Crate Selection

**Audio decoding & playback**: `rodio` (high-level API) backed by `symphonia` (pure Rust, supports all target formats). `rodio` handles device output via `cpal` internally.

**BPM detection**: `stratum-dsp` — pure Rust, zero FFI, designed for DJ applications, achieves 87.7% accuracy within ±2 BPM on real-world tracks. Uses a dual tempogram approach (FFT + autocorrelation). Offers beat tracking, not just BPM estimation.

**TUI**: `ratatui` with `crossterm` backend.

### Architecture

Three concurrent components communicate via shared state:

```
┌─────────────────┐     Arc<SharedState>     ┌──────────────────┐
│  Audio Thread   │ ─────────────────────── │   TUI Thread     │
│                 │                          │                  │
│  rodio Sink     │  - playback_pos: AtomicU64 (samples elapsed)│
│  custom Source  │  - is_playing: AtomicBool│  ratatui render  │
│  (counts samples│  - bpm: f32 (set once)   │  beat flash      │
│   → atomic pos) │  - duration: u64 samples │  loop ~60fps     │
└─────────────────┘                          └──────────────────┘
         ▲
         │ decoded at startup
┌─────────────────┐
│  Analysis       │
│  (main thread,  │
│  before play)   │
│  symphonia      │
│  → PCM samples  │
│  → stratum-dsp  │
│  → BPM result   │
└─────────────────┘
```

### Startup Sequence
1. Parse CLI arg → file path.
2. Decode full audio to PCM using `symphonia` (needed for BPM analysis).
3. Feed PCM to `stratum-dsp` → get BPM.
4. Construct a `rodio` custom `Source` wrapping the decoded samples that atomically increments a sample counter on each `next()` call.
5. Spawn audio thread: push source into `rodio::Sink`, begin playback.
6. Run TUI render loop on main thread.

### Beat Flash Logic
Given BPM and sample rate:
- `beat_period_samples = sample_rate * 60.0 / bpm`
- Current beat phase = `playback_pos_samples % beat_period_samples`
- Flash when phase is within a small window (e.g. first 100ms of beat)

### TUI Layout (spike — minimal)
```
┌─────────────────────────────┐
│  tj — <filename>            │
│                             │
│  BPM: 128.0                 │
│                             │
│       [ BEAT ]              │  ← flashes on each beat
│                             │
│  [Playing]  00:32 / 03:45   │
│                             │
│  Space: play/pause   q: quit│
└─────────────────────────────┘
```

### Key Risks & Mitigations
- **BPM accuracy**: `stratum-dsp` is the best available pure-Rust option. If accuracy is insufficient on evaluation tracks, fallback options are `aubio-rs` (C bindings, battle-tested) or manual implementation.
- **Playback position tracking**: Custom `Source` wrapper avoids rodio's lack of a native position API. Simple and low-overhead.
- **Full decode on load**: For large files this could be slow to start. Acceptable for a spike; streaming decode is a later concern.

## Tasks
1. ✓ **Setup**: Initialise Rust project (`cargo init`), add dependencies (`rodio`, `symphonia`, `stratum-dsp`, `ratatui`, `crossterm`).
2. ✓ **Impl**: Audio decode — read file with symphonia, produce interleaved f32 PCM samples.
3. ✓ **Impl**: BPM detection — feed PCM to stratum-dsp, extract BPM value.
4. ✓ **Impl**: TUI — ratatui render loop, timer-driven beat flash (no audio output), display BPM; quit.
5. ✓ **Pause**: Assess container audio environment; set up ALSA/PulseAudio if needed before proceeding.
6. ✓ **Impl**: Playback — `rodio` 0.22 API (`DeviceSinkBuilder`/`Player`); `player.get_pos()` used for beat phase (no atomic counter needed); play/pause; phase offset fine-tune (+/-).
7. ✓ **Verify**: Tested with fixture WAVs and a real FLAC (Boloria, 120 BPM → detected 119.78). BPM accuracy confirmed good. Beat flash + audio playback working on Ubuntu 24.04 host.
8. ✓ **Process**: Confirm ready to archive.
