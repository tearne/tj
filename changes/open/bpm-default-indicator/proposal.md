# Proposal: BPM Default Indicator
**Status: Draft**

## Problem

When a track is loaded with no cache entry, BPM is set to 120.0 as a placeholder. The info bar displays this the same way as a confirmed BPM, giving the user no signal that the value is meaningless.

## Goal

Make the unconfirmed state visually distinct — ideally at a glance, without reading numbers.

## Options

### A — Dim colour
Render the BPM value in a muted colour (e.g. dark grey) when unconfirmed, normal colour once confirmed. Simple. No text changes.

### B — Question-mark suffix
Show `120?` instead of `120.0` when unconfirmed. Clear meaning, low noise.

### C — Tilde prefix
Show `~120` instead of `120.0`. Conventional "approximately" notation.

### D — Dashes
Show `---` instead of a number. Unambiguous: there is no BPM. Requires tap or redetect to proceed.

## Recommendation

**Option B** (`120?`) — it preserves the numeric value (useful if the user happens to be playing at 120) while clearly flagging uncertainty. The `?` is conventional for "unverified."

## Implementation Notes

- Add a `bpm_confirmed: bool` field to `TempoState` (or similar), defaulting to `false`.
- Set to `true` on: cache hit, BPM tap (after 8 taps), redetect confirmation.
- The info bar formats BPM as `{:.1}?` when `!bpm_confirmed`, `{:.1}` when confirmed.
- The playback-speed BPM (f/v adjustment) may also benefit from the indicator, since adjusting an unconfirmed BPM is less meaningful.
