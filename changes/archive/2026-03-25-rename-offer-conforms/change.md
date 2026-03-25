# Rename Offer Conforms Check
**Type**: Fix
**Status**: Approved

## Problem

`build_deck` suppresses the rename offer whenever `stem_conforms(stem)` is true — i.e. the filename already contains ` - `. This means a file named `Artist - Track Name` never gets an offer, even though the proposed convention is `<title> - <artist>`, which would produce `Track Name - Artist`.

## Fix

Replace the `stem_conforms` gate with a direct comparison of the current stem against `propose_rename_stem`. Offer the rename iff they differ. When there are no usable tags, `propose_rename_stem` falls back to the current stem, so `proposed == stem` and no offer is made — existing behaviour for untagged files is preserved.

## Test Gap

The `rename_roundtrip` test and `propose_rename_stem_non_empty` test were written against the old `stem_conforms` logic and do not verify that filename text actually matches tags.

**`rename_roundtrip` — first pass**: uses `stem_conforms` to skip files whose stem already contains ` - `, identical to the production bug. Files with a conforming stem are copied unchanged without calling `propose_rename_stem`, so mismatched content like `Artist - Track Name` is never caught.

**`rename_roundtrip` — idempotency pass**: skips conforming stems with `if !stem_conforms(...)`, so the idempotency assertion is never exercised on any file that already contained ` - `. After the first-pass fix, every output file is either already at its proposed name or has been renamed — the idempotency check should apply to all files unconditionally.

**`propose_rename_stem_non_empty`**: only asserts the result is non-empty. Does not verify the result contains ` - ` or that the parts match the track's artist and title tags.

## Proposed Test Fixes

**First pass**: replace the `stem_conforms` branch with `propose_rename_stem`, mirroring production:
```rust
let target_stem = propose_rename_stem(src);
```
The stem is always the proposed name; no special case for conforming stems.

**Idempotency pass**: remove the `stem_conforms` guard and assert unconditionally:
```rust
let proposed = propose_rename_stem(dst);
assert_eq!(proposed, current_stem, ...);
```

**`propose_rename_stem_non_empty`**: after calling `propose_rename_stem`, read artist and title tags independently via `read_tags_for_editor`. If both are present assert the stem equals `"{sanitised_title} - {sanitised_artist}"` exactly. If tags are absent fall back to the existing non-empty assertion.

## Log

- `src/main.rs`: replaced `stem_conforms` gate with `propose_rename_stem(path) != stem` comparison; removed `stem_conforms` import
- `src/tags/mod.rs`: removed `stem_conforms` entirely (no longer used in production or tests); `rename_roundtrip` first pass always calls `propose_rename_stem`; idempotency assertion now unconditional; `propose_rename_stem_non_empty` asserts exact `<title> - <artist>` format when both tags are present
