#!/usr/bin/env bash
# Flamingo — grade the 2026-08-06 recapture, new arm vs ctrl arm, same binary.
#
# Runs the two CANONICAL graders Sun used for the master table
# (`grade_gate.py` = G3/G5/G6, `grade_axes.py --profile gameplay` = the P0 axes)
# plus `measure_penumbra.py` for G4a, on all eight frames. `grade_g3.py` is run
# on top because G3 is the scorecard's #1 gap and its deep-dive columns
# (p05-L + darkest-shade hue) are what the ambient patch is aimed at.
#
# Output is one section per frame so the two arms can be diffed by eye and by
# grep; nothing here decides pass/fail on its own — the graders do.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${OUT:-_fl_recap_20260806}"

for stem in gate3-boot gate3-walk gate3-combat grade-vista; do
  for arm in new ctrl; do
    f="$OUT/${stem}-${arm}-nohud2.png"
    echo "################################################################"
    echo "# $stem  [$arm]   $f"
    echo "################################################################"
    if [ ! -s "$f" ]; then echo "MISSING"; echo; continue; fi
    echo "---- grade_gate (G3/G5/G6) ----"
    python scripts/grade_gate.py "$f" 2>&1 | tail -20
    echo "---- grade_axes (P0, gameplay profile) ----"
    python scripts/grade_axes.py --profile gameplay "$f" 2>&1 | tail -14
    echo "---- grade_g3 (p05-L + darkest shade) ----"
    python scripts/grade_g3.py "$f" 2>&1 | grep -iE "p05-L|p01|p10|darkest|RGB=|warm" | head -8
    echo "---- penumbra (G4a) ----"
    python scripts/measure_penumbra.py "$f" 2>&1 | grep -iE "median|penumbra|px" | head -4
    echo
  done
done
echo "=== recap grade done ==="
