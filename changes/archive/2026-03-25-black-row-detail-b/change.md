# Black Row Below Deck B Detail Waveform
**Type**: Fix
**Status**: Archived

## Problem

A single black row appears between the bottom of the deck B detail waveform and deck A's notification bar. It is always present regardless of terminal height or detail zoom level.

## Cause

The root asymmetry: deck A's layout area comprised `h-1` waveform rows + 1 shared tick row appended inside `render_detail_waveform`. Deck B's area was the same height `h` but had no tick row, leaving one row unfilled. The `Paragraph` widget left that row as blank braille with no styling — terminal default black.

Attempts at minimal fixes (extending the buffer to `h` rows; clamping `buf_r` to repeat the last row) either shifted the zero-line off-centre for deck A or produced a visible glitch at the bottom of deck B.

## Fix

Extract the shared tick row from `render_detail_waveform` into its own layout slot between the two detail areas. Both waveform areas are now the same height and both buffers use the same row count. The asymmetry is gone.

- `src/main.rs`: `fixed` incremented from 6 to 7; layout gains a dedicated tick slot (`area_tick`) between `area_detail_a` and `area_detail_b`; `shared_renderer.rows` stores `h` (not `h-1`); tick extraction moved out of the deck A render block into a shared block before both decks
- `src/render/mod.rs`: `render_detail_waveform` loses `shared_tick` parameter and tick-append logic; `waveform_rows` is always `detail_panel_rows`; new `render_shared_tick_row` renders the tick line into the dedicated area slot
