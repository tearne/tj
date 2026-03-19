# Proposal: Split Cue Mode
**Status: Approved**

## Intent

When DJing with headphones and a single audio output, the DJ needs to listen to an incoming track privately before bringing it in. Split cue mode enables this by routing Deck A to the left channel and Deck B to the right channel, so both decks can be monitored independently in headphones — no second output device required.

## Specification Deltas

### ADDED

**Audio — Split Cue Mode** (`SPEC/audio.md`)

- A global `split_cue` toggle (default off) is activated by `\`. It is not persisted between sessions.
- While split cue is active:
  - Deck A audio is routed to the left channel only (right channel zeroed).
  - Deck B audio is routed to the right channel only (left channel zeroed).
  - Both decks play at full volume (level multiplier fixed at 1.0); the stored `level` value is unaffected and takes effect again when split cue is turned off.
  - Both decks play unfiltered (`filter_offset` treated as 0); IIR state is maintained so there is no click on deactivation. The stored `filter_offset` is unaffected.

**Keymap — Global Controls** (`SPEC/keymap.md`)

| Key | Action |
|-----|--------|
| `\` | Toggle split cue mode on / off |

**Layout — Global Status Bar** (`SPEC/layout.md`)

- While split cue is active, a `[split cue]` label is shown in the global status bar in amber. Content priority becomes: system notification > split cue indicator > idle status.

### MODIFIED

- **Global status bar content priority** (`SPEC/layout.md`): previously system notification > idle status; now system notification > split cue indicator > idle status.

## Scope

- **In scope**: the single-output split cue monitor described above.
- **Out of scope**: per-deck cue routing (PFL), a dedicated house/master output (deferred to a future change). When a house output is added, split cue will apply only to the monitor/cue output and have no effect on the house output.
