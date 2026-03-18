# Design: Cue Points

## New Fields

### `Deck`
```rust
cue_sample: Option<usize>,       // cue point in audio samples; None = not set
cue_held: bool,                  // hold-to-play is active (key currently held)
cue_tap_pending: Option<Instant>, // waiting for second BPM tap to confirm cue clear
cue_offset_pending: Option<Instant>, // waiting for second offset press to confirm cue clear
```

Initialised to `None`, `false`, `None`, `None` in `Deck::new`.

### `CacheEntry`
```rust
#[serde(default)]
cue_sample: Option<usize>,
```

## New Actions

```rust
Deck1Cue, Deck2Cue,      // Space+A, Space+D
Deck1CueSet, Deck2CueSet, // A (Shift+a), D (Shift+d)
```

Added to the action table and default config.

## Key Handler

Cue keys must be handled at **all** `KeyEventKind` variants (like nudge), not filtered to `Press` only. The outer `if key.kind != KeyEventKind::Press { continue; }` guard must be bypassed for these actions.

### `Deck1Cue` / `Deck2Cue` — Press

```
let current_samp = d.display.smooth_display_samp as usize;

if d.audio.player.is_paused() {
    if let Some(cue) = d.cue_sample {
        // hold-to-play: seek to cue and play
        seek(cue)
        play
        d.cue_held = true
    } else {
        // no cue set: set cue at current position
        d.cue_sample = Some(current_samp)
    }
} else {
    // playing: set new cue at current position and pause
    d.cue_sample = Some(current_samp)
    pause
}
```

### `Deck1Cue` / `Deck2Cue` — Release

```
if d.cue_held {
    d.cue_held = false
    pause
    if let Some(cue) = d.cue_sample {
        seek(cue)
    }
}
```

### `Deck1CueSet` / `Deck2CueSet` — Press

```
d.cue_sample = Some(d.display.smooth_display_samp as usize)
// persist to cache
```

`KeyEventKind::Release` and `Repeat` are ignored for these.

## BPM Tap Confirmation

In the `Deck1BpmTap` / `Deck2BpmTap` handler, before registering the tap:

```rust
if d.cue_sample.is_some() {
    if d.cue_tap_pending
        .map_or(false, |t| t.elapsed().as_secs_f64() < 5.0)
    {
        // confirmed: clear cue and proceed with tap
        d.cue_sample = None;
        d.cue_tap_pending = None;
        // fall through to normal tap logic
    } else {
        // first press: set pending, notify, skip tap
        d.cue_tap_pending = Some(Instant::now());
        set_notification("BPM tap will clear the cue point — tap again to confirm", Warning, 5s);
        return; // don't register the tap
    }
}
```

In `service_deck_frame`, expire `cue_tap_pending` after 5 seconds (clear the flag if elapsed; the notification expires independently).

## Offset Adjustment Confirmation

Same pattern for the `+` / `_` offset keys. Before applying the offset change:

```rust
if d.cue_sample.is_some() {
    if d.cue_offset_pending
        .map_or(false, |t| t.elapsed().as_secs_f64() < 5.0)
    {
        d.cue_sample = None;
        d.cue_offset_pending = None;
        // fall through to normal offset logic
    } else {
        d.cue_offset_pending = Some(Instant::now());
        set_notification("Offset change will clear the cue — press again to confirm", Warning, 5s);
        return;
    }
}
```

## BPM Anchoring (f/v keys)

After any `base_bpm` or `bpm` change, if a cue is set, re-anchor `offset_ms` so the beat grid stays aligned to the cue position:

```rust
if let Some(cue_samp) = d.cue_sample {
    let cue_ms = cue_samp as f64 / d.audio.sample_rate as f64 * 1000.0;
    let beat_period_ms = 60000.0 / d.tempo.base_bpm as f64;
    let raw = cue_ms.rem_euclid(beat_period_ms);
    d.tempo.offset_ms = (raw / 10.0).round() as i64 * 10;
}
```

## Cache

On track load, restore `d.cue_sample` from the entry's `cue_sample` field (if present).

On cue set or clear, persist immediately:
```rust
cache.set(hash, CacheEntry { cue_sample: d.cue_sample, ..entry });
cache.save();
```

On quit, persist `cue_sample` alongside BPM and offset.

## Visual Markers

### Overview (`overview_for_deck`)

Compute the cue column from the cue sample fraction:

```rust
let cue_col: Option<usize> = deck.cue_sample.map(|samp| {
    let frac = (samp as f64 / deck.audio.sample_rate as f64
        / deck.total_duration.as_secs_f64()).clamp(0.0, 1.0);
    ((frac * overview_width as f64).round() as usize)
        .min(overview_width.saturating_sub(1))
});
```

In the per-column match, add before the spectral colour fallthrough:

```rust
} else if cue_col == Some(c) {
    (Color::Green, '│')
}
```

### Detail waveform (`render_detail_waveform`)

Compute the cue screen column in the same coordinate system as beat ticks. The `view_start` and `half_samples_per_col` values are already available in the function:

```rust
let cue_screen_col: Option<usize> = deck.cue_sample.and_then(|samp| {
    let disp_half = ((samp as f64 - view_start) / half_col_samp_global).round() as i64;
    if disp_half >= 0 {
        let col = (disp_half / 2) as usize;
        if col < detail_width { Some(col) } else { None }
    } else {
        None
    }
});
```

In the per-column colour selection for **all rows** (tick and waveform), add after the centre-column check:

```rust
} else if cue_screen_col == Some(c) {
    (Color::Green, '│')
}
```

This gives the cue marker higher priority than tick marks but lower than the playhead.

## `service_deck_frame` Changes

Expire confirmation flags after 5 seconds:

```rust
if d.cue_tap_pending.map_or(false, |t| t.elapsed().as_secs_f64() >= 5.0) {
    d.cue_tap_pending = None;
}
if d.cue_offset_pending.map_or(false, |t| t.elapsed().as_secs_f64() >= 5.0) {
    d.cue_offset_pending = None;
}
```
