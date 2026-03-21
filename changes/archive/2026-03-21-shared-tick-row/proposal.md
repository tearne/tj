# Proposal: Shared Tick Row
**Status: Approved**

## Problem

Each detail waveform currently devotes two rows to tick marks (top and bottom), leaving only `detail_height − 2` rows for actual waveform content. At the default `detail_height = 6` that is 4 waveform rows and 4 tick rows total across both decks — the tick rows consume as much vertical space as the waveform.

## Proposed Change

Replace the four per-deck tick rows (2 × top, 2 × bottom) with a **single shared tick row** positioned between the two detail waveforms. This row renders both decks' beat grids simultaneously, in different colours, saving 3 rows overall.

### Layout change

```
Before (current):
  [ tick A top    ]   ← 1 row
  [ waveform A    ]   ← detail_height - 2 rows
  [ tick A bottom ]   ← 1 row
  [ notif A       ]
  [ info A        ]
  [ overview A    ]
  [ tick B top    ]   ← 1 row
  [ waveform B    ]   ← detail_height - 2 rows
  [ tick B bottom ]   ← 1 row
  ...

After:
  [ waveform A    ]   ← detail_height - 1 rows  (+1 waveform row per deck)
  [ shared ticks  ]   ← 1 row  (replaces 4 rows with 1)
  [ notif A       ]
  [ info A        ]
  [ overview A    ]
  [ waveform B    ]   ← detail_height - 1 rows  (+1 waveform row per deck)
  ...
```

Net saving: **3 rows** (4 tick rows → 1 shared tick row).

### Shared tick row rendering

The shared tick row sits at the bottom of the Deck A detail area. Both Deck A and Deck B tick grids are OR'd together into a single braille byte array and rendered in the existing tick grey (`Color::Gray`). No colour distinction between decks — if their grids differ, you see the union; if they share the same BPM and offset, the marks coincide exactly.

The shared row is part of the Deck A detail panel in the layout (it occupies its bottom row). Deck B's detail panel gains the row that was previously its top tick row as additional waveform space.

### BrailleBuffer changes

- `shared_renderer.rows` stores `h − 1` for both decks (one fewer tick row each).
- The shared tick row is rendered in the UI thread in screen space from both decks' `tick_display` vectors, exactly as tick rows are computed today.

### `detail_height` semantics

`detail_height` changes meaning: it becomes the total rows of the Deck A detail panel including the shared tick row (i.e. `detail_height − 1` waveform rows). `DET_MIN` drops from 4 to 3 (1 shared tick row + 2 waveform rows minimum per deck). The config comment and SPEC are updated accordingly.

## Considerations

- The shared row is always associated with the Deck A panel in the layout. If Deck A is empty but Deck B is loaded, the shared tick row still renders (showing only Deck B ticks).
- Cue marker visibility: the cue marker currently uses the top and bottom tick rows to stay visible when the playhead overlaps the cue column. With the new layout, the top row of each deck's waveform panel becomes the outer edge; the cue marker logic updates accordingly.

## Risk

Low. The tick computation is unchanged; only the layout mapping and row count change.
