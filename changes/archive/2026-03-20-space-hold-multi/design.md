# Design: Space Held for Multiple Key Presses
**Status: Archived**

## Approach

Three independent changes, each touching a small surface area.

### 1. Remove post-chord `space_held` resets (conditional on config)

Three sites currently reset `space_held = false` after a chord fires:
- Line ~715: `Deck1Cue` space-chord handler
- Line ~736: `Deck2Cue` space-chord handler
- Line ~909: general `SpaceChord` action resolver

Replace each bare reset with a conditional:
```rust
if display_cfg.space_chord_auto_reset { space_held = false; }
```
`space_held` then persists across chord presses until the Space Release event clears
it, unless `space_chord_auto_reset = true` opts back into the old behaviour.

### 2. `space_chord_auto_reset` config option

Add `space_chord_auto_reset: bool` to `DisplayConfig` (default `false`). Parse from
the `[keys]` section in `parse_display_config`. Add a commented-out entry with
explanation to `resources/config.toml`.

```toml
[keys]
# space_chord_auto_reset = true  # set if your terminal does not send Space key-release events
```

`parse_display_config` reads `parsed.get("keys")` for this field (the `[display]` and
`[keys]` sections are both available in the parsed TOML value).

### 3. `[SPC]` indicator in the detail info bar

The draw closure already captures `space_held` by reference. Extend the detail info bar
format string to append `  [SPC]` when `space_held` is true:

```rust
format!("  zoom:{}s  lat:{}ms{}",
    zoom_secs, audio_latency_ms,
    if space_held { "  [SPC]" } else { "" })
```

## Tasks

1. Impl: add `space_chord_auto_reset: bool` to `DisplayConfig`; parse from `[keys]`
   in `parse_display_config`; add commented entry to `resources/config.toml`
2. Impl: replace three `space_held = false` resets with conditional on
   `display_cfg.space_chord_auto_reset`
3. Impl: append `  [SPC]` to detail info bar when `space_held` is true
4. Verify: `cargo build` clean; manual test — hold Space, press two chord keys in
   sequence, confirm both fire; confirm `[SPC]` appears and clears correctly
5. Process: confirm ready to archive
