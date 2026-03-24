# Findings: Filename Convention

## What Was Established

**Convention** — `Track Name - Artist Name [(Suffix)]`. The stem must contain ` - `
with non-empty parts on either side. A trailing parenthetical is permitted. Conformance
is checked against the raw filename stem, not the tag-derived display name.

**Rename offer** — On load of a non-conforming file, a timed `rename? [y/h]` offer
appears in red for 10 s then dims but persists. `y` accepts the proposed stem directly;
`h` opens the tag editor; all other keys pass through.

**Tag editor** — Centered modal with seven editable fields (Artist, Title, Album, Year,
Track, Genre, Comment), pre-populated from the file's embedded tags. Blue/navy/yellow
colour scheme. Section dividers separate the tag fields from the filename preview.
Fields wrap at the popup width. Up/Down and Tab/Shift-Tab navigate between fields.
Enter confirms (blocked if Artist or Title is blank); Esc cancels. All field values are
trimmed on confirm.

**Proposed stem** — Built as `sanitise_for_filename(title) - sanitise_for_filename(artist)`.
Illegal filename characters (`/ \ : * ? " < > |`) are replaced with `-`. Falls back to
the current stem when tags are absent.

**Tag reading** — Uses symphonia. `collect_tags` checks probe-level metadata first
(ID3v2 on MP3) then format-level (ID3v1, Vorbis Comments), so richer tags win when
both are present.

**Rename roundtrip test** — `rename_roundtrip` (requires `TEST_AUDIO_DIR`) copies all
audio files to a temp directory under proposed names, then asserts count and total size
are preserved and the operation is idempotent. Validated against 122 real tracks.

## What Was Deferred

- **Disk write** — No tags are written to files and no files are renamed. `rename_accepted`
  is populated but not consumed. This requires a tag-writing library (symphonia is
  read-only) and a collision guard before `fs::rename`.

- **Remaining test gaps** — `sanitise_for_filename` edge cases; collision scenario;
  `propose_rename_stem` never returning an empty string.

- **Post-rename state** — `d.path` and `d.filename` must be updated after a rename so
  subsequent operations use the new path.

All deferred items are captured in the `tag-write-rename` proposal.

## Key Decisions

- Convention is `Title - Artist`, not `Artist - Title`, matching how tracks are
  typically identified by their title first.
- Conformance check is intentionally loose (`stem_conforms` only requires ` - ` with
  non-empty parts) so that files already in a reasonable format are not re-renamed.
- The tag editor edits all seven common fields but only Artist and Title affect the
  proposed filename; the others are stored ready for the write path.
