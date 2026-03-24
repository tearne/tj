# Filename Convention
**Type**: Spike
**Status**: Archived

## Goal

Establish a naming convention for audio files, surface a rename invitation when a
loaded file does not conform, and provide an in-place workflow (tag editing → rename)
to correct files. The exact UX flow is being discovered through iteration. A disk write
is deliberately deferred until proposals can be verified correct in practice.

## Convention

```
Track Name - Artist Name [(Suffix)]
```

The stem (filename without extension) must contain ` - ` with non-empty parts on
either side. Any trailing parenthetical is treated as a suffix.

## Log

**Conformance check and proposed stem** — At load time `stem_conforms()` checks the
raw filename stem (not the tag-derived display name). If non-conforming,
`propose_rename_stem()` reads ID3 artist + title tags and produces `Artist - Title`
(ASCII hyphen); falls back to the raw stem if tags are absent.

**Initial key scheme (`/`)** — First iteration: persistent `rename? [/]` hint in the
notification line. Pressing `/` showed the proposed stem with a 10 s countdown; any
key dismissed. Replaced because the timeout-on-any-key pattern was too aggressive and
the `/` key felt arbitrary.

**Timed offer with y/h keys** — Second iteration: rename offer starts immediately on
load, shows `rename? [y/h]  (10s)` in red for 10 seconds then dims to DarkGray and
persists. `y` accepts (shows `→ stem` notification, clears offer), `h` opens tag
editor, all other keys pass through without touching the offer.

**Tag editor modal** — Pressing `h` opens a centered overlay titled
`Edit tags and rename file`. Shows Artist and Title editable fields pre-populated from
the proposed stem, with `Current: filename.ext` and `Proposed: new-name.ext` (live
preview). Tab/Shift-Tab switch fields; standard cursor keys and text editing apply.
Enter confirms (updates proposed stem, shows brief notification), Esc cancels.

**Modal isolation** — Mouse clicks and all key events (including space, nudge, BPM
ramp) are blocked from reaching the rest of the UI while the tag editor is open. The
tag editor intercept is placed before space tracking and nudge handling in the event
loop.

**Pre-write bug fixes** — `collect_tags` now prefers probe-level metadata (ID3v2) over
format-level (ID3v1), so richer tags win on files carrying both. Enter is blocked when
artist or title is blank; on confirm, all field values are trimmed in place. Empty file
extension no longer produces a trailing dot in the current/proposed display.

**Rename roundtrip test** — `rename_roundtrip` in `#[cfg(test)]` reads all audio files
from `TEST_AUDIO_DIR` (skips if unset), copies each to a temp directory under its
proposed name, then asserts: (1) output count and total byte size match the input;
(2) re-running `propose_rename_stem` on every output file proposes no further changes.
Naming collisions are caught and reported. Temp directory is cleaned up on drop.

**Full tag set** — Editor expanded from two fields (Artist, Title) to seven: Artist,
Title, Album, Year, Track, Genre, Comment. Tags are read directly from the file via
symphonia on open (replacing the hint-parsing approach). `TagEditorState` uses a
`Vec<(String, usize)>` for values and cursors, indexed by `active_field`. Tab/Shift-Tab
cycle forward and backward through all seven fields. Popup height increased from 9 to
14 rows.

## Outcome

Adopted. The spike established the UX flow, the convention, and the full tag editor.
All code is in production. Disk write (tag update + file rename) is deferred to the
`tag-write-rename` proposal, which also covers collision prevention, post-rename state
update, and the remaining test gaps identified during review.

## On Archive

Create `SPEC/tags.md` covering the tag and file-handling aspects of the application:
the naming convention, conformance check, rename workflow, tag editor UX, and the
write path (tag update + file rename). This work is too large and behaviour-rich to
remain undocumented only in `SPEC.md`.
