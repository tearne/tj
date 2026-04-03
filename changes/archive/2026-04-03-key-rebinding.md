# Key Rebinding

## Intent

Cluster related controls onto adjacent keys so that related actions sit under the same finger. Nudge and tick offset adjustment (used together for beat alignment) move to D/C. BPM rate controls (playback and native) move to F/V, forming a column directly below Play/Pause (Space+F) — making F the anchor of all tempo-sensitive actions. BPM establishment tools (tap and re-detect) consolidate onto B. Metronome follows BPM+ to Space+V, keeping monitoring adjacent to adjustment.

The keyboard help overlay is also updated: action labels adopt a consistent `+/-` prefix notation; `F` and its `╭`/`╰` brackets are rendered in green as the anchor key; and the five primary performance actions — `Play`, `+Ndge`, `-Ndge`, `+BPM`, `-BPM` — are highlighted in green.

Full overlay layout after changes:

```
╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭ +Slp    ╭ +Slp
1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7 HPF     8 HPF
╰ SelD1   ╰ SelD2   ╰         ╰         ╰         ┆   ╰ Flt=    ╰ Flt=
  ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭ -Slp    ╭ -Slp
  Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U LPF     I LPF
  ╰         ╰         ╰ CueSt   ╰ CueJp   ╰         ┆   ╰ Flt=    ╰ Flt=
    ╭         ╭         ╭ +Tick   ╭ -BsBPM  ╭         ┆   ╭ +Gain   ╭ +Gain
    A -Ptch   S +PFL    D +Ndge   F -BPM    G         ┆   J +Lvl    K +Lvl
    ╰ -Ptch   ╰ Rst     ╰ Brows   ╰ Play    ╰ PFLTog  ┆   ╰ 100%    ╰ 100%
      ╭         ╭         ╭ -Tick   ╭ +BsBPM  ╭         ┆   ╭ -Gain   ╭ -Gain
      Z +Ptch   X -PFL    C -Ndge   V +BPM    B Tap     ┆   M -Lvl    , -Lvl
      ╰ +Ptch   ╰ Rst     ╰         ╰ Metro   ╰ BDtct   ┆   ╰ 0%      ╰ 0%
```

Green highlighting: `F` label and its `╭`/`╰` brackets are bold green. The five primary performance actions — `+Ndge`, `-Ndge`, `-BPM`, `+BPM`, `Play` — are rendered in green.

`SPEC/config.md` is also updated to present the keyboard layout using the `╭`/`╰` style established by the in-app keyboard help overlay, replacing the current ASCII table format.

## Approach

### Binding changes

Eleven bindings change in `src/config/mod.rs` and `resources/config.toml`:

| Function | Old key | New key |
|---|---|---|
| `nudge_forward` | `f` | `d` |
| `nudge_backward` | `v` | `c` |
| `tick_increase` | `F` | `D` |
| `tick_decrease` | `V` | `C` |
| `bpm_decrease` | `g` | `f` |
| `bpm_increase` | `b` | `v` |
| `base_bpm_decrease` | `G` | `F` |
| `base_bpm_increase` | `B` | `V` |
| `bpm_tap` | `c` | `b` |
| `bpm_detect` | `space+c` | `space+b` |
| `metronome` | `space+b` | `space+v` |

`g`, `G`, `C` (uppercase), and `space+c` become unbound.

### Overlay rendering

`render_keyboard_help` (introduced by the keyboard-layout-help change, which must be built first) is updated:

- All action labels use `+/-` prefix notation throughout (e.g. `Fwd` → `+Ndge`, `Ptch-` → `-Ptch`, `PFL+` → `+PFL`, `Lvl+` → `+Lvl`, `Gain+` → `+Gain`, `Slp+` → `+Slp`).
- The `F` cell (key label and its `╭`/`╰` brackets) is rendered bold green.
- Five action labels are rendered green: `+Ndge`, `-Ndge`, `-BPM`, `+BPM`, `Play`.

### SPEC/config.md

The keyboard layout table is rewritten in the `╭`/`╰` overlay style (matching the sketch in the Intent), replacing the current ASCII box-drawing table. The legend section is updated to reflect the new bindings.

Review cadence: at the end. Requires keyboard-layout-help to be built first.

## Plan

- [x] UPDATE `src/config/mod.rs`: change the eleven bindings listed in the Approach
- [x] UPDATE `resources/config.toml`: update the same eleven bindings
- [x] UPDATE `render_keyboard_help` in `src/render/mod.rs`: apply `+/-` prefix notation to all action labels and add green highlighting for `F` cell and the five primary action labels
- [x] UPDATE `SPEC/config.md`: rewrite keyboard layout in `╭`/`╰` style and update legend for new bindings

## Log

- Post-plan overlay refinements: `Brows` added as a highlighted action; `F` key label and its `╭`/`╰` brackets changed from bold green to white; `Brows` and `Play` changed to blue (Space-modifier indicator). `bg_gr` style and `Modifier` import removed as no longer needed.

## Conclusion

Delivered as planned. Eleven bindings relocated (nudge → D/C, tick offset → D/C Shift, BPM rate → F/V, base BPM → F/V Shift, tap → B, redetect → Space+B, metronome → Space+V). Overlay updated with `+/-` prefix notation throughout. Final highlighting: `F` key and its brackets white; primary performance action labels (`+Ndge`, `-Ndge`, `-BPM`, `+BPM`) green; Space-modifier actions (`Brows`, `Play`) blue. `SPEC/config.md` keyboard layout rewritten in `╭`/`╰` style with a condensed legend.
