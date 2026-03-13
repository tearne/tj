# Proposal: Style Conformance
**Status: Draft**

## Intent

Address style guide gaps not already captured in the code review or multi-deck structural findings. The single-file monolith (S1–S4) will be resolved as part of multi-deck; this change targets the remaining violations.

## Specification Deltas

No user-visible behaviour changes. All deltas are internal code quality.

### MODIFIED

- **Error types (STYLE-RUST P6)**: `Box<dyn Error>` is replaced with domain error types at module boundaries. `tui_loop`'s return type is clarified. `thiserror` is used for structured errors; `anyhow` is used at the application entry point.

- **Naming (STYLE P4)**: Abbreviated identifiers that obscure intent are renamed to intention-revealing alternatives. Examples: `spc` → `samples_per_col`, `samp` → `samples`, `bpm_rx` → `bpm_result_rx`.

## Scope

- **In scope**: error type definitions, renaming of non-obvious abbreviations, removal of any comments that merely restate what code does (STYLE P5 cleanup).
- **Out of scope**: structural decomposition of `tui_loop`, module extraction — these are prerequisites of `multi-deck`.

## Notes

- S1–S4 structural findings (orchestration, abstraction levels, module boundaries) are already captured as `multi-deck` prerequisites and are not repeated here.
- This change should be applied before multi-deck structural work begins, so renamed identifiers don't need to be tracked through a large refactor.
