# Design: Tap BPM Realtime Ticks

## Root Cause

There are two tap update paths:

**1. Realtime path** — fires on every tap once `tap_times.len() >= 8`, inside the key handler:
```rust
if d.tap.tap_times.len() >= 8 {
    let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
    d.tempo.base_bpm = tapped_bpm;
    d.tempo.bpm = ...;
    d.tempo.offset_ms = tapped_offset;
    d.audio.player.set_speed(...);
    shared_renderer.store_speed_ratio(...);
    // ← bpm_established NOT set here
}
```

**2. Session-end path** — fires in `service_deck_frame` when the 2-second timeout expires:
```rust
if d.tap.was_tap_active && !tap_active_now && d.tap.tap_times.len() >= 8 {
    // ... same updates ...
    d.tempo.bpm_established = true;  // ← set here
}
```

The tick marks and BPM colour both gate on `bpm_established`:

- `analysing = spinner_active || !d.tempo.bpm_established` — controls tick visibility in the detail waveform.
- `unconfirmed = !d.tempo.bpm_established` — controls the red/normal BPM colour in the info bar.

Until the session ends and `service_deck_frame` sets `bpm_established = true`, the realtime BPM updates are applied to `base_bpm`/`offset_ms` (audio is correct) but the render treats the deck as still-analysing.

## Change

Add `d.tempo.bpm_established = true` in the realtime tap path:

```rust
if d.tap.tap_times.len() >= 8 {
    let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
    d.tempo.base_bpm = tapped_bpm;
    d.tempo.bpm = ...;
    d.tempo.offset_ms = tapped_offset;
    d.tempo.bpm_established = true;  // ← add this
    d.audio.player.set_speed(...);
    shared_renderer.store_speed_ratio(...);
}
```

Same addition in both the Deck 1 and Deck 2 tap handlers.
