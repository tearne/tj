# Filename Convention
**Type**: Proposal
**Status**: Draft

## Overview

Establish a naming convention for audio files and surface a rename invitation in the
deck notification line when a loaded file does not conform.

## Convention

```
Artist Name - Track Name [(Suffix)]
```

Where `Suffix` is optional and covers: `Person Remix`, `Extended`, `Mixed`, or any
free-form parenthetical comment. The stem (filename without extension) must contain
` - ` with non-empty artist and title on either side.

A library-scan mode is a possible future extension — point tj at a directory of
well-named files and infer the pattern in use. For this change, the convention above
is hardcoded.

## Conformance Check

At load time, check the raw filename stem (not the tag-derived `track_name`) against
the convention. A file conforms if its stem matches:

```
<artist> - <title> [(<suffix>)]
```

i.e. contains ` - ` with non-empty strings on both sides, and any trailing parenthetical
is well-formed. Files loaded from a path that has no filename (edge case) are treated
as conforming.

## UI

When the loaded file does not conform, the notification line shows a right-justified
rename invitation alongside the track name on the left:

```
Artist Name - Track Name                    rename? [r]
```

The notification line currently renders a single left-aligned span. This change
introduces a left/right spacer pattern (matching the info line) so both sides can
coexist. The rename prompt is styled dimly (DarkGray) so it does not compete with the
track name or override higher-priority notifications (BPM pending, active_notification).

Priority order in the notification line remains unchanged: BPM-pending prompt >
active_notification > track name. The rename hint is only visible when the track name
is showing (i.e. no higher-priority notification is active).

## Rename Action

Pressing `r` when a non-conforming file is loaded opens an inline text input in the
notification line, pre-populated with a suggested stem derived from the file's ID3
tags (`Artist - Title`) if available, or the current filename stem otherwise. Confirming
renames the file on disk and reloads `filename` and `track_name` on the deck. Cancelling
(`Esc`) dismisses without change.

Open question: whether a successful rename should clear the hint permanently for that
file (it will, because the new name will conform) or whether to also persist a
"user acknowledged" flag to suppress the hint even if the user chose not to rename.

## Log
