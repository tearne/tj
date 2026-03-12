# Design: Metronome
**Status: Draft**

## Approach

### State
Add `metronome_mode: bool` (default `false`) and `last_metro_pulse: Option<Instant>`.
Reset both to `false`/`None` when a new track is loaded.

### Toggle key
Add `Action::MetronomeToggle` to the `Action` enum and `ACTION_NAMES`. Add `metronome_toggle = "'"` to `config.toml`. Handler: flip `metronome_mode`; clear `last_metro_pulse` on deactivate.

### Click firing
In the main loop, after the calibration pulse block, add a metronome pulse block:

```rust
if metronome_mode && !calibration_mode {
    let beat_period = 60.0 / bpm as f64;
    let fire = match last_metro_pulse {
        None => true,
        Some(t) => t.elapsed().as_secs_f64() >= beat_period,
    };
    if fire {
        play_click_tone(mixer, sample_rate);
        last_metro_pulse = Some(Instant::now());
    }
} else {
    last_metro_pulse = None;
}
```

This reuses `play_click_tone` exactly as the calibration pulse does. The `beat_period` is derived from the current `bpm` (which includes any `f`/`v` adjustment), so tempo changes take effect on the next beat. Phase alignment: on activation, the first click fires immediately, then subsequent clicks follow the beat period — this gives instant feedback without needing to sync to `offset_ms` wall-clock position.

### Info bar indicator
In the left group BPM rendering, append `♪` in red (`Color::Red`) immediately after the BPM value when `metronome_mode`:

```rust
if metronome_mode {
    spans.push(Span::styled("♪", Style::default().fg(Color::Red)));
}
```

### Help overlay
Add `'              toggle metronome` line.

## Tasks

1. **Impl**: Add `MetronomeToggle` to `Action` enum, `ACTION_NAMES`, `config.toml`, and handler.
2. **Impl**: Add metronome pulse firing in the main loop; reset on track load.
3. **Impl**: Add `♪` indicator in info bar left group.
4. **Impl**: Update help text.
5. **Verify**: Click fires at BPM rate. Indicator appears/disappears. Tempo changes reflected. Resets on track load. No click during calibration.
6. **Process**: Confirm ready to archive.
