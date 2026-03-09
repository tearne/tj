# Design: Remove Render Modes
**Status: Draft**

## Approach

Remove the `live_mode` boolean, the `m` key binding, and all associated branching. The background thread retains its existing buffer-mode behaviour unchanged. The `live` variable passed into the buffer computation block (which currently affects buffer width: 2× in live, 5× in buffer) is removed; buffer width is always 5×.

Affected code:
- `live_mode_shared` Arc<AtomicBool> and `live_mode` local — remove both
- Background thread `live` load and the `if live { recompute always }` branch — remove
- `buf_cols` conditional (`if live { cols * 2 } else { cols * 5 }`) — replace with `cols * 5`
- `m` key handler in the event loop — remove
- Key hints string — remove `m: mode(...)` entry

SPEC.md: remove the `### Detail Waveform Render Modes` section; update the detail waveform description to drop any mode references.

## Tasks
1. ✓ Impl: Remove `live_mode`, `live_mode_shared`, `m` key handler, and all live-mode branching from `src/main.rs`
2. ✓ Impl: Update SPEC.md — remove render modes section, tidy references
3. ✓ Verify: buffer-mode rendering unchanged; `m` key inert or unbound
4. ✓ Process: confirm ready to archive
