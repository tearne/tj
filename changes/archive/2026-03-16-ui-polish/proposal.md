# Proposal: UI Polish
**Status: Draft**

## Intent

Three small visual improvements to clean up the UI now that the two-deck model is fully established.

---

## 1 — Global notification bar: move to top

### Current
The global notification bar (`area_global`) is the bottom-most row in every layout variant — positioned after deck B's overview. It only appears when there is a notification to show (pending quit confirm, startup hint, config warning), so its position is inconsistent with its role as a high-priority system message.

### Change
Move `area_global` to the first row of the layout, above `area_detail_info`. Every layout variant ([true, true], [true, false], [false, true]) is updated the same way.

New constraint order (both render paths):
```
0: global bar         ← moved here from last
1: detail info bar
2: detail A
3: detail B (if both)
4: notif A
5: info A
6: overview A
7: notif B (if both)
8: info B (if both)
9: overview B (if both)
```

---

## 2 — Info bar label colour: remove active-deck highlight

### Current
Active deck label (`A` / `B`) is rendered in the deck's spectral palette colour (`active_label_style = Color::Rgb(tr, tg, tb)`). Inactive deck label uses dim grey (`Color::Rgb(70, 70, 70)`). This asymmetry was meaningful when one deck was "selected"; it no longer is.

### Change
Remove `active_label_style` / `dim_label_style` distinction for label colours. Use a single uniform dark blue for both labels: `Color::Rgb(40, 60, 100)` — distinct from surrounding content, consistent with a header/structural role, and not tied to deck state.

This applies to: notification bar labels, info bar labels, and any empty-deck placeholder labels in both render paths.

---

## 3 — Analysis spinner: slow down

### Current
`SPINNER[frame_count % SPINNER.len()]` — advances one frame per render tick. At ~60 fps this completes one full rotation every ~170 ms, which is visually frantic.

### Change
Advance the spinner every N frames: `SPINNER[(frame_count / 6) % SPINNER.len()]`. At 60 fps this gives ~1 rotation per second — readable and calm.

The divisor `6` is a constant; adjust in code if the target rate needs tuning after testing.
