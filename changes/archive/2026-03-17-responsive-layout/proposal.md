# Proposal: Responsive Detail Height
**Status: Implemented — v0.5.80**

## Problem

When the terminal window is too short to accommodate the full layout, the detail waveforms keep their configured height and the rows below them (info bars, overviews, global bar) are clipped or get zero height. The compaction is abrupt and irregular — the bottom of the UI simply falls off.

The full layout at default settings requires approximately 26 rows:

| Section | Rows |
|---|---|
| Global bar | 1 |
| Detail info bar | 1 |
| Detail A | 6 (configurable) |
| Detail B | 6 (configurable) |
| Notif A | 1 |
| Info A | 1 |
| Overview A | 4 |
| Notif B | 1 |
| Info B | 1 |
| Overview B | 4 |
| **Total** | **26** |

## Goal

When the terminal shrinks, detail waveforms compress first (down to a minimum), keeping the info bars and overviews visible for as long as possible.

## Proposed Behaviour

Inside the draw closure, compute an `effective_det_h` clamped to `[DET_MIN, detail_height]` based on the actual inner height available:

```rust
const DET_MIN: usize = 3;
let fixed_rows = 10; // all non-detail rows (global, detail_info, notif×2, info×2, overview×2)
let available_for_detail = (inner.height as usize).saturating_sub(fixed_rows);
let effective_det_h = (available_for_detail / 2).clamp(DET_MIN, detail_height);
```

The layout then uses `effective_det_h` in place of `detail_height`. The user-configured `detail_height` is unchanged — it remains the ceiling, not a hard request.

## What Changes

- One local variable `effective_det_h` computed inside the draw closure.
- The two `Constraint::Length(det_h)` calls use `effective_det_h` instead.
- The `SharedDetailRenderer` row store uses `effective_det_h` (already inside the draw closure).
- The render functions receive `effective_det_h` where they need row count.

## What Does Not Change

- `detail_height` config and `{`/`}` key behaviour.
- All other constraints (overviews stay fixed at 4, info/notif stay at 1).
- When the terminal is large enough, `effective_det_h == detail_height` — no visible difference.

## Edge Case

If the terminal is so small that even `DET_MIN * 2 + fixed_rows` rows aren't available, `effective_det_h` stays at `DET_MIN` and the lower sections begin to clip. This is unavoidable but at least the compaction priority is correct.

## Risk

Low. The change is confined to the layout computation inside the draw closure. No data structures or logic are affected.
