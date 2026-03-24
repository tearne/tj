# Tag Write and File Rename
**Type**: Proposal
**Status**: Draft

## Intent

The filename-convention spike established the full UX workflow and left `rename_accepted` populated but unconsumed — no tags are written and no files are renamed. This proposal completes the write path and closes the remaining test gaps identified during the pre-write review.

## Specification Deltas

### ADDED

**Tag write** — On confirmation, the seven tag fields edited in the tag editor are written back to the file's embedded tags using a format-aware library. Symphonia is read-only; a suitable write-capable library must be introduced (e.g. `lofty`, which handles MP3/ID3, FLAC/Vorbis, and other formats through a single API).

**File rename** — After a successful tag write, the file is renamed in place to `sanitised_stem.extension`. A rename is only attempted when the proposed stem differs from the current stem.

**Collision guard** — When the user presses Enter in the tag editor, the proposed target path is checked for existence before the dialogue closes. If a collision is detected, an inline error is shown inside the modal and the editor remains open so the user can amend their edits without losing them. The rename is only attempted after the dialogue closes with a confirmed, collision-free stem.

**Post-rename state update** — After a successful rename, `d.path` and `d.filename` are updated to the new path so that all subsequent operations (tag editor re-open, cache write, display) use the correct location.

**Tests: tag write** — Integration test that writes a known set of tag values to a copy of a real audio file, reads them back via `collect_tags`, and asserts every field matches. Covers at least one MP3 and one FLAC file to verify format-specific write paths independently.

**Tests: sanitise_for_filename** — Unit tests covering: a string composed entirely of illegal characters produces a non-empty result (all replaced with `-`); an empty string input returns an empty string; a string with no illegal characters is returned unchanged.

**Tests: propose_rename_stem** — Unit test asserting the function never returns an empty string, given a path whose stem is non-empty.

**Tests: collision detection** — Integration test asserting that when the proposed stem collides with an existing file, the tag editor remains open, an inline error is visible, and both files are left intact.

### MODIFIED

**`rename_accepted`** — Currently set but never consumed. After this change it is consumed by the write path: tag update followed by file rename.

## Test Fixtures

Integration tests that operate on real audio files require a fixture directory supplied via the `TEST_AUDIO_DIR` environment variable. Tests skip silently when the variable is unset so that `cargo test` passes without fixtures present. The fixture directory for this project is `/root/test_tracks`, containing 122 files in MP3 and FLAC formats organised in year/letter subdirectories, scanned recursively.

## Scope

- **In scope**: tag write, file rename, collision guard, post-rename state update, the three sets of tests above.
- **Out of scope**: batch rename of multiple files.
