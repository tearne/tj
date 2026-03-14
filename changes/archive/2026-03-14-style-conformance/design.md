# Design: Style Conformance
**Status: Draft**

## Approach

Two categories of work, kept as separate tasks for easy review.

### Naming (STYLE P4)

Only identifiers that obscure intent to a reader unfamiliar with the local context. Domain-conventional abbreviations in DSP code (`x`, `y`, `k`, `db`, `l`, `r` for filter/braille internals) are left as-is — they are standard in their domain.

High-value renames:

| Old | New | Location |
|-----|-----|----------|
| `spc` | `samples_per_col` | detail waveform rendering |
| `dw` | `detail_width` | detail waveform rendering |
| `dh` | `detail_height_px` | detail waveform rendering (avoids clash with `detail_height` usize config var) |
| `ow` | `overview_width` | overview waveform rendering |
| `oh` | `overview_height` | overview waveform rendering |
| `vs` | `viewport_start` | buffer viewport slicing |
| `ds` | `decoded_samples` | decode thread closure in `main` |
| `et` | `estimated_total` | decode thread closure in `main` |

`dim` (a named `Style` constant) and `bpm_rx`/`bpm_tx` (idiomatic Rust channel naming) are intentionally left unchanged.

### Error types (STYLE-RUST P6)

The two `decode_audio` and `Cache::load` functions return `Box<dyn std::error::Error>` — the catch-all that erases error information. Since this is a binary (not a library), `color-eyre` is the right tool: it preserves context via `.wrap_err()` chains, and its colourised output is visible after `cleanup_terminal()` restores the terminal.

Changes:
- Add `color-eyre = "0.6"` to `Cargo.toml`
- Call `color_eyre::install()` at the top of `main` before any fallible operations
- Replace `Box<dyn std::error::Error>` return types with `color_eyre::Result<_>`
- Replace bare string error constructions with `eyre::eyre!()` or `bail!()`
- `io::Result` function signatures (terminal setup, browser, tui_loop) are left as-is — these genuinely wrap IO operations and `io::Result` is idiomatic there

`unwrap()` call sites are left for a future pass — most protect invariants that would require a broader refactor to express properly, and changing them without that context risks replacing silent panics with noisy ones.

## Tasks

1. ✓ **Impl**: Rename abbreviations (`spc`, `dw`, `dh`, `ow`, `oh`, `vs`, `ds`, `et`)
2. ✓ **Impl**: Add `color-eyre`, replace `Box<dyn Error>` in `decode_audio` and `detect_bpm`
3. **Process**: Build clean, ready to archive
