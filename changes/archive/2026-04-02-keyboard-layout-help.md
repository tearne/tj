# Keyboard Layout Help

## Intent
Players who are learning the controls need a quick visual reference they can keep on screen while playing. The existing modal help (`?`) blocks the mental model of the layout by presenting bindings as a list rather than as a spatial keyboard. A persistent, non-modal keyboard map rendered in the album art panel — showing keys in their physical positions with their actions annotated — lets users build spatial memory of the controls while the music is still running.

## Approach

The keyboard help is rendered in the album art panel (`c[11]`, the leftover spacer rows between the two decks). It is non-modal — it does not intercept input or alter key handling.

### Layout

The control keys divide into two physical groups matching their position on a standard keyboard: a left block (deck controls, keys 1–5 / Q–T / A–G / Z–B) and a right block (mixer, keys 7–8 / U–I / J–K / M–,). Keys 6, Y, H, and N are intentionally unbound and are omitted. Global keys (`` ` ``, `-`, `=`, `[`, `]`, etc.) are few enough to remain covered by the existing `?` modal only.

Each keyboard row is rendered as three text lines — Shift layer above, key letter plus plain action on the key line, Space layer below. The `|` separator character appears only on the Shift and Space rows (left-aligned to each cell), keeping the key row clean. Cells with no binding on a modifier row are left blank between separators. This gives 12 lines total across the four keyboard rows:

```
╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭ Slp+    ╭ Slp+
1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7 HPF     8 HPF
╰ Sel D1  ╰ Sel D2  ╰         ╰         ╰         ┆   ╰ Flt=    ╰ Flt=
  ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭ Slp-    ╭ Slp-
  Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U LPF     I LPF
  ╰         ╰         ╰ CueSt   ╰ CueJp   ╰         ┆   ╰ Flt=    ╰ Flt=
    ╭         ╭         ╭         ╭ +Tick   ╭ -BsBPM  ┆   ╭ Gain+   ╭ Gain+
    A Ptch-   S PFL+    D         F Fwd     G -BPM    ┆   J Lvl+    K Lvl+
    ╰ Ptch=   ╰ Rst     ╰ Brows   ╰ Play    ╰ PFLTog  ┆   ╰ 100%    ╰ 100%
      ╭         ╭         ╭         ╭ -Tick   ╭ +BsBPM  ┆   ╭ Gain-   ╭ Gain-
      Z Ptch+   X PFL-    C Tap     V Back    B +BPM    ┆   M Lvl-    , Lvl-
      ╰ Ptch=   ╰ Rst     ╰ BDtct   ╰         ╰ Metro   ┆   ╰ 0%      ╰ 0%
```

### Toggle

`Space+/` shows and hides the keyboard layout. This is independent of the album art cycle (`/`) — both can coexist, and the keyboard layout takes precedence over album art when active.

### Modal help (`?`) — trimmed

All keys covered by the keyboard layout are removed from the `?` modal. Only global keys that do not appear in the layout remain, plus a prompt directing the user to `Space+/`:

```
── Global ───────────────────────────────────────────────────────────────
`                    vinyl mode
¬                    nudge mode toggle
- / =                zoom in / out
{ / }                waveform height
[ / ]                latency ±10ms
/                    album art                    ~  palette cycle
Space+/              keyboard layout
?                    toggle this help
Esc                  close this / quit
```

### Sizing

When the keyboard layout is active it requests 12 rows in the spacer area. The existing deck compression logic responds as it does for any space pressure — detail waveform heights shrink first, then overview heights — to try to free the required rows. Whatever space is actually available is given to the layout; lines that don't fit are clipped at the bottom rather than suppressed. The layout always renders top-down, so the most important rows (number row, QWERTY) appear first.


## Plan

Review cadence: at the end.

- [x] ADD `KeyboardHelp` action variant to the `Action` enum in `src/config/mod.rs`
- [x] UPDATE `resources/config.toml`: add `keyboard_help = "space+/"` binding
- [x] ADD `keyboard_help_open: bool` state in `tui_loop` in `src/main.rs`
- [x] UPDATE layout sizing in `tui_loop`: when `keyboard_help_open`, add 12 to the `fixed` row count so deck compression makes room for the spacer
- [x] ADD `render_keyboard_help(frame, area)` in `src/render/mod.rs` producing the staggered 12-line layout from the Approach sketch
- [x] UPDATE main render loop: in the spacer branch, render keyboard help when `keyboard_help_open` (takes precedence over album art), otherwise existing art logic unchanged
- [x] UPDATE `Action::KeyboardHelp` handler in the key event loop to toggle `keyboard_help_open`
- [x] UPDATE `?` modal help string in `src/main.rs`: remove all keys covered by the layout, add `Space+/` line, matching the trimmed text in the Approach
- [x] UPDATE `SPEC/config.md`: add `Space+/` to the keyboard layout table and legend
- [x] UPDATE `SPEC/render.md`: document the keyboard help panel, sizing behaviour, and `Space+/` toggle

## Log

- Overlay approach: after review, the sizing pressure (+12 to `fixed`) was removed and the keyboard help was changed to render as an overlay on top of the album art rather than replacing it. Art renders first; help is drawn on top centred horizontally at the top of the spacer.
- Dark background: `Clear` followed by `Block::style(bg)` used to fill the outer rect. `Block::style` alone only calls `set_style` (changes colour attributes but leaves halfblock art characters in place), causing a comb edge artifact. `Clear` resets cells to plain spaces first.
- Margin: 2-col / 1-row padding added around the text inside the dark bg rect, implemented by splitting the outer rect (filled by Block) from the inner rect (filled by Paragraph) rather than using ratatui Padding.
- 1-row top offset: box shifted down one row to clear the gap between deck 2 and the art area.
- Crash on small terminal: `outer_h` was clamped to `area.height` without accounting for the 1-row offset, so `outer.y + outer.height` could exceed the buffer. Fixed by clamping to `area.height.saturating_sub(1)` with an early return when nothing fits.
- Version: released as 0.8.3 (initial), 0.8.4 (overlay + margin), 0.8.5 (Clear fix + crash fix).

## Conclusion

Delivered as planned. `Space+/` toggles a non-modal 12-line keyboard map overlaid on the album art in the spacer panel; the `?` modal is trimmed to global-only keys with a pointer to `Space+/`. Post-plan: rendering changed to overlay (art visible around the box), halfblock comb artifact fixed with `Clear`, 2-col/1-row margin added, 1-row top offset applied, and a buffer-overflow crash on small terminals fixed.


