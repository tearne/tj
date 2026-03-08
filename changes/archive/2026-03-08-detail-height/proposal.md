# Proposal: Configurable Detail Waveform Height
**Status: Ready for Review**

## Intent

The detail waveform currently occupies all remaining vertical space after the fixed UI elements. On large terminals this causes the braille grid (rows × buf_cols cells) to grow large enough to produce visible stutter, because even a cheap O(cols × rows) loop becomes expensive when rows is very large.

The user should be able to cap the detail view height at a comfortable size and change it at runtime.

## Specification Deltas

### ADDED

- The detail waveform height is user-controllable at runtime:
  - `↑` / `↓` (or another key pair, TBD) increase / decrease the height by one terminal row.
  - The height is bounded between a minimum of 1 row and a maximum equal to the available vertical space.
  - The current height is remembered for the session; the default on launch is 8 rows.
- The height is applied as a fixed `Constraint::Length` on the detail waveform panel; any unused space below the panel is left blank.

### MODIFIED

- **Waveform Visualisation**: The detail view height is no longer determined solely by the layout; it is capped by the user-configured value.
