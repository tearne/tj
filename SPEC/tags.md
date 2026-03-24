# Specification: Tags and File Naming

## Overview

Tracks carry embedded metadata tags (artist, title, album, etc.). The application
enforces a filename convention derived from those tags and provides an in-app workflow
to correct non-conforming files without leaving the TUI.

## Naming Convention

```
Track Name - Artist Name [(Suffix)]
```

The filename stem (without extension) must contain ` - ` (space-hyphen-space) with a
non-empty part on each side. A trailing parenthetical suffix is permitted. Conformance
is checked against the raw filename stem, not the display name derived from tags.

## Behaviour

### Conformance Check

At load time, `stem_conforms(stem)` is evaluated on the raw filename stem. Files that
already conform are loaded silently. Files that do not conform trigger the rename offer.

### Rename Offer

When a non-conforming file is loaded:

- A `rename? [y]` notice appears in the deck's notification row in red.
- The offer is active for 10 seconds, after which it dims to DarkGray but persists.
- `y` — opens the tag editor, pre-populated from the file's embedded tags.
- All other keys pass through to normal deck controls without dismissing the offer.

### Proposed Stem

The proposed stem is built as:

```
sanitise_for_filename(title) - sanitise_for_filename(artist)
```

where `sanitise_for_filename` replaces characters illegal or ambiguous in filenames
(`/ \ : * ? " < > |`) with `-`. If either tag is absent, the current filename stem is
used as the fallback; no rename is proposed.

### Tag Editor

`h` opens a centred modal dialogue with seven editable fields:

| Field   | Tag              |
|---------|------------------|
| Artist  | Artist           |
| Title   | Track title      |
| Album   | Album            |
| Year    | Date             |
| Track   | Track number     |
| Genre   | Genre            |
| Comment | Comment          |

Fields are pre-populated from the file's embedded tags. The dialogue shows a live
preview of the proposed filename beneath the fields.

- Tab / Shift-Tab / Up / Down — navigate between fields.
- Standard cursor keys, Backspace, Delete, Home, End — edit within a field.
- Enter — confirm; blocked if Artist or Title is blank. All field values are trimmed on
  confirm.
- Esc — cancel without changes.

While the tag editor is open, all other key and mouse events are blocked.

### Write Path

On confirmation, the tag editor:

1. Writes the updated field values back to the file's embedded tags.
2. Renames the file to `sanitised_stem.extension` in the same directory.

Before renaming, the application checks that the target path does not already exist. If
it does, the rename is aborted and an error notification is shown.

After a successful rename, the deck's internal path and filename are updated to reflect
the new location.

### Tag Reading

Tags are read via symphonia. Probe-level metadata (ID3v2 on MP3) is preferred over
format-level metadata (ID3v1, Vorbis Comments) when both are present.

## Constraints

- The write path requires a tag-writing library; symphonia is read-only.
- A file rename is only attempted when the proposed stem differs from the current stem.
- Filenames with characters illegal on the target filesystem are sanitised before write.
- The rename operation must not overwrite an existing file.

## Verification

- `rename_roundtrip` integration test: copies all files in `TEST_AUDIO_DIR` to a temp directory under proposed names; asserts count and total byte size are preserved and the operation is idempotent across a second pass.
- Unit tests: `sanitise_for_filename` edge cases; `propose_rename_stem` never returns an empty string; collision detection prevents overwrite.
