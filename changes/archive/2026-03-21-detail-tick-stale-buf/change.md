# Fix: Detail Tick Marks Disappear on Resize (Deck B)
**Type**: Fix

## Log

Tick marks intermittently disappear from deck B's detail waveform when resizing.

**Root cause**: `half_col_samp_global` in `render_detail_waveform` is initialised to `1.0` and only
set as a side-effect of the `viewport_start` block, which is skipped when the buffer is stale
(`buf.buf_cols < detail_width`). The tick computation then uses `1.0` as the half-column sample
count, placing every tick thousands of columns off-screen.

Deck B is more often affected because the background thread writes `shared_a` before `shared_b`,
leaving a one-frame window where deck A has the refreshed buffer but deck B does not.

**Fix**: Compute `half_col_samp_global` directly from `buf.samples_per_col` before the
`viewport_start` block, decoupling it from viewport availability.
