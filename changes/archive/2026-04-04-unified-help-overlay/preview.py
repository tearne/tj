#!/usr/bin/env -S uv run --script --
# /// script
# requires-python = "==3.12.*"
# ///

"""Render the proposed unified help overlay to stdout for colour-scheme review."""

import argparse
import os
import sys

VERSION = "1.0.0"

AMBER      = "\033[38;2;130;100;50m"
GRAY       = "\033[38;2;170;170;170m"
BLUE       = "\033[38;2;60;100;160m"
WHITE      = "\033[37;1m"
GREEN_BOLD = "\033[38;2;80;140;70m"
RESET      = "\033[0m"
BG         = "\033[48;2;15;15;15m"


def render_overlay():
    for row in overlay_rows():
        print(BG + "  " + row + RESET)


def overlay_rows():
    return [
        # 1  Shift
        a("╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭ +Slp    ╭ +Slp    ╭ +Slp"),
        # 2  Bare
        g("1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7 HPF     8 HPF     9 HPF"),
        # 3  Space
        b("╰ SelD1   ╰ SelD2   ╰ SelD3   ╰         ╰         ┆   ╰ Flt=    ╰ Flt=    ╰ Flt="),
        # 4  Shift
        a("  ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭ -Slp    ╭ -Slp    ╭ -Slp"),
        # 5  Bare
        g("  Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U LPF     I LPF     O LPF"),
        # 6  Space
        b("  ╰         ╰         ╰ CueSt   ╰ CueJp   ╰         ┆   ╰ Flt=    ╰ Flt=    ╰ Flt="),
        # 7  Shift  (F ╭ bracket stays white)
        a("    ╭         ╭         ╭ +Tick   ") + w("╭") + a(" -BsBPM  ╭         ┆   ╭ +Gain   ╭ +Gain   ╭ +Gain"),
        # 8  Bare   (F key name stays white; +Ndge / -BPM stay green bold)
        g("    A -Ptch   S +PFL    D ") + gr("+Ndge") + g("   ") + w("F") + g(" ") + gr("-BPM") + g("    G         ┆   J +Lvl    K +Lvl    L +Lvl"),
        # 9  Space  (F ╰ bracket stays white)
        b("    ╰ -Ptch   ╰ Rst     ╰ Brows   ") + w("╰") + b(" Play    ╰ PFLTog  ┆   ╰ 100%    ╰ 100%    ╰ 100%"),
        # 10 Shift
        a("      ╭         ╭         ╭ -Tick   ╭ +BsBPM  ╭         ┆   ╭ -Gain   ╭ -Gain   ╭ -Gain"),
        # 11 Bare   (-Ndge / +BPM stay green bold)
        g("      Z +Ptch   X -PFL    C ") + gr("-Ndge") + g("   V ") + gr("+BPM") + g("    B Tap     ┆   M -Lvl    , -Lvl    . -Lvl"),
        # 12 Space
        b("      ╰ +Ptch   ╰ Rst     ╰         ╰ Metro   ╰ BDtct   ┆   ╰ 0%      ╰ 0%      ╰ 0%"),
        # 13 Separator + legend
        g("──────────────────────────────────────────────────────────────  [") + AMBER + "Shift" + RESET + g("]  [") + GRAY + "Bare" + RESET + g("]  [") + BLUE + "Space" + RESET + g("]"),
        # 14 Global keys (bare)
        g("` vinyl   ¬ nudge   -/= zoom   {/} height   [/] latency   Esc quit"),
        # 15 Global keys (bare)
        g("/ art   ~ palette   Spc+= swap1↔2   Spc+- swap2↔3"),
    ]


def a(t): return AMBER      + t + RESET
def g(t): return GRAY       + t + RESET
def b(t): return BLUE       + t + RESET
def w(t): return WHITE      + t + RESET
def gr(t): return GREEN_BOLD + t + RESET


def main():
    parser = argparse.ArgumentParser(description="Render unified help overlay preview")
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    parser.parse_args()

    print()
    render_overlay()
    print()


if __name__ == "__main__":
    if not os.environ.get("VIRTUAL_ENV"):
        print("Error: no virtual environment detected. Run this script via './preview.py' (requires uv), or activate a virtual environment first.")
        sys.exit(100)
    main()
