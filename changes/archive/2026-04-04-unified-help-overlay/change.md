# Unified Help Overlay

## Intent

The app currently has two separate help surfaces: a `?` prose modal listing global keys, and a `Space+/` keyboard diagram overlay. Users need to switch between them to get a complete picture of the key bindings. This change replaces both with a single unified overlay, triggered by `?`, that shows all key bindings in one diagram and includes a modifier legend. Colour is used to distinguish plain, Shift, and Space-modifier layers within the diagram.

## Approach

The overlay stays in its current position — the spacer area (`c[16]`) over the album art — and is slightly taller. `?` opens it; `Space+/` is removed. The existing `?` prose modal is deleted.

### Layout (inner area, 15 rows × 87 cols)

Rows 1–12: existing keyboard diagram, with systematic colour applied by layer (see below).

Row 13: separator with the modifier legend flush-right:

```
  ──────────────────────────────────────────────────────  [Shift]  [Bare]  [Space]
```

Rows 14–15: global keys not shown in the diagram:

```
  ` vinyl   ¬ nudge   -/= zoom   {/} height   [/] latency   Esc quit
  / art   ~ palette   Spc+= swap1↔2   Spc+- swap2↔3
```

### Colour scheme

Three modifier layers, each with a distinct colour applied consistently:

- **Shift layer** (`╭` rows and their labels): dim warm amber — `Rgb(130, 100, 50)`
- **Bare layer** (key-name rows and their labels): medium gray — `Rgb(170, 170, 170)`
- **Space layer** (`╰` rows and their labels): dim cool blue — `Rgb(60, 100, 160)`

Structural characters (`╭`, `╰`, bracket lines, `┆`) take the colour of their layer. The legend on the separator row shows "Shift", "Bare", "Space" in the corresponding colours.

Existing exceptions are preserved: F-key bracket stays white; nudge and BPM labels stay green bold. Global key rows use Bare colour.

### Preview script

This change is folder-format (`unified-help-overlay/change.md` + `preview.py`). The builder writes `preview.py` first: a self-contained Python 3 script using ANSI escape codes that renders the full proposed overlay to stdout so the colour scheme can be reviewed before any source changes are made.

Review cadence: after the preview script (colour scheme sign-off required before proceeding), then at the end.

## Plan

- [x] CONVERT change to folder format: move `unified-help-overlay.md` → `unified-help-overlay/change.md`
- [x] ADD `changes/active/planning/unified-help-overlay/preview.py`: Python 3 script rendering the full proposed overlay with ANSI colours — all 15 inner rows, separator with legend, global key rows
- [x] REVIEW colour scheme with user before continuing
- [x] UPDATE `src/render/mod.rs`: extend `render_keyboard_help` — increase `TEXT_H` to 15, add separator row with legend, add two global key rows; apply systematic layer colouring throughout
- [x] UPDATE `src/main.rs`: bind `?` to the overlay; remove `Space+/` binding and its state variable; remove the old `?` modal block
- [x] UPDATE `resources/config.toml`: remove `keyboard_layout` binding (was `space+/`)
- [x] UPDATE `SPEC/config.md`: reflect unified overlay on `?`, remove `Space+/` entry, update global keys list

## Log

- User requested `preview.py` be rewritten as a POS script (uv shebang, VERSION, argparse --version, main guard with venv check).
- Green nudge/BPM labels toned down: removed bold, switched to muted sage `Rgb(80, 140, 70)` in both preview and source.
- Bug found: "press any key to dismiss" block from the old `?` modal was still in place, swallowing height-change keypresses. Replaced with Esc-only close.
- User requested default detail waveform height one setting shorter: `detail_height` default 6 → 5 (both `DisplayConfig::default()` and parser fallback).

## Conclusion

All tasks completed. The two help surfaces (`?` modal and `Space+/` keyboard diagram) are replaced by a single unified overlay on `?`. Layer colouring (amber/gray/blue) applied throughout; nudge/BPM labels use muted sage `Rgb(80, 140, 70)` rather than bold green. `KeyboardHelp` action and its state variable removed from all locations.
