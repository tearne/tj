# Proposal: Overview Waveform Alternative Rendering
**Status: Note**

## Intent
The overview uses braille by convention only. An alternative character set may produce a nicer result.

## Attempted: Eighth-block characters (`▁`–`█`)
Block chars were tried but did not look as good as braille — the solid fill was visually inferior to the dot texture. Reverted. A different approach is needed before this moves forward.

## Unresolved
- What alternative would actually look better? Options to consider:
  - Keep braille but adjust colour/contrast rather than the character set.
  - Explore a different character set (e.g. half-blocks, shading chars `░▒▓█`).
  - Accept braille and close this proposal.
