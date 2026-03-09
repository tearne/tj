# Design: Nudge
**Status: Draft**

## Approach

`Player::set_speed(f32)` is available in rodio 0.22. It works by inflating the declared `sample_rate` of the source, causing rodio's resampler to pull samples faster/slower. This correctly advances the `TrackingSource` position atomic at the new rate, so position tracking and drift correction both work without modification. Pitch shifts with speed — this is the intended DJ-deck behaviour.

Detecting key hold requires `KeyEventKind::Release` events, which crossterm only emits when keyboard enhancement is enabled (`PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES)`). Terminals that do not support the protocol silently ignore the flag; no existing behaviour changes.

Existing key handlers must be guarded to `Press | Repeat` kinds only, so that releasing any key does not re-trigger its action.

### Nudge keys
`,` — nudge backward (0.9×)
`.` — nudge forward (1.1×)

### Playing
- On `,`/`.` `Press` or `Repeat`: call `player.set_speed(0.9)` / `player.set_speed(1.1)`.
- On `,`/`.` `Release`: call `player.set_speed(1.0)`.

### Paused
`set_speed` has no effect while paused (rodio is not pulling samples). Instead, each render frame while nudging and paused:
- Advance `smooth_display_samp` by `elapsed * sample_rate * ±0.1`.
- Clamp to `[0, total_samples]`.
- Call `seek_direct(smooth_display_samp / sample_rate)` to sync the actual position atomic.

On nudge release while paused, the position is already synced by the per-frame call; no extra action needed.

### UI
Show `[nudge ▶]` or `[nudge ◀]` in the status line when active, replacing the `[Playing]`/`[Paused]` text prefix (or appended alongside it — simpler).

## Tasks
1. ✓ Impl: Enable/disable `PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES)` in terminal setup and teardown
2. ✓ Impl: Guard existing key handlers to `Press | Repeat` kind; add `,`/`.` press/release handlers to set nudge state and call `player.set_speed()`
3. ✓ Impl: Paused nudge — per-frame position drift when paused and nudging
4. ✓ Impl: UI — append nudge indicator to status line when active
5. ✓ Verify: nudge forward/backward while playing shifts speed and pitch; release restores 1.0×; nudge while paused drifts the display position; UI indicator appears and disappears correctly
6. Process: confirm ready to archive
