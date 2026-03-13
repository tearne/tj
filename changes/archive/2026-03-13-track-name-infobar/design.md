# Design: Track Name Info Bar
**Status: Approved**

## Approach

### Layout change

The current inner layout has three rows:

```
Constraint::Length(1), // info bar      ← chunks[0]
Constraint::Length(4), // overview      ← chunks[1]
Constraint::Min(0),    // detail + gap  ← chunks[2]
```

A fourth row is inserted above the info bar:

```
Constraint::Length(1), // track name    ← chunks[0]  (new)
Constraint::Length(1), // info bar      ← chunks[1]
Constraint::Length(4), // overview      ← chunks[2]
Constraint::Min(0),    // detail + gap  ← chunks[3]
```

All downstream chunk indices shift by one.

### Track name content

Symphonia's `probed.format.metadata()` exposes ID3/Vorbis/etc. tags via
`current()` → `tags()`. We look for `StandardTagKey::TrackTitle` and
`StandardTagKey::Artist`. If both are present the bar shows `artist – title`;
if only a title is present it shows the title; otherwise it falls back to the
filename (current behaviour of the window title).

The track name is computed once at decode time (before `tui_loop`) and passed
in as a `String`. `decode_audio` is extended to return this alongside the
existing four values, or a separate small helper reads the tags.

Simplest approach: add a `fn read_track_name(path: &str) -> String` that
re-probes the file (probe is fast; no decoding), reads tags, and returns the
display string. Called in `main` after `decode_audio`, before launching
`tui_loop`.

### Frame / window title

- `outer` block title: `format!(" tj {} ", env!("CARGO_PKG_VERSION"))`
- Terminal window title (line ~2847): `format!(" tj {} ", env!("CARGO_PKG_VERSION"))` — already uses `tj — cwd`, update to version form.

## Tasks

1. ✓ **Impl**: Add `fn read_track_name(path: &str) -> String` — probe file, read
   `StandardTagKey::Artist` / `StandardTagKey::TrackTitle`; return
   `"artist – title"`, `"title"`, or filename fallback.
2. ✓ **Impl**: Pass `track_name: String` into `tui_loop`; add `Constraint::Length(1)`
   row above info bar; render track name as a dim `Paragraph` in `chunks[0]`;
   shift all chunk indices.
3. ✓ **Impl**: Simplify frame border title and terminal window title to
   `tj vX.Y.Z`.
4. **Process**: Verify build, ready to archive.
