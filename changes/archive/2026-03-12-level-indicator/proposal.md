# Proposal: Refine Level Indicator
**Status: Note**

## Intent
Polish the level indicator display.

## Changes

### Remove space before bracket
`level: ▕▃▏` → `level:▕▃▏` (colon directly followed by bracket, no space).

### Lighter bracket colour
Currently `▕` and `▏` are rendered in the same `dim` style as surrounding text. Explore whether a slightly lighter or distinct colour for the brackets makes the indicator read more clearly as a self-contained widget — e.g. a mid-grey that stands out from the dim label but doesn't compete with the amber spectrum strip.

## Unresolved
- What colour works well for the brackets? Options: white at reduced brightness, a neutral grey, or matching the beat-flash amber at low intensity.
