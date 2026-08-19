#!/usr/bin/env bash
# ===========================================================================
# Flamingo — PILE A follow-up #2: the white balance, on a FULL GRID.
#
# WHY THIS EXISTS: MY FIRST HYPOTHESIS WAS WRONG.
#   I blamed the flat AMBIENT fill for the blue room and swept it 3900 -> 1800
#   lux with two AMBCOLOR values (scripts/_fl_v4_ambladder_20260818.sh, all six
#   rungs kept in _fl_v4_20260818/ambladder). Cool % went 63.1 -> 60.3. The fill
#   is not what turned the room blue; killing it costs brightness and buys back
#   under three points of cool. Hypothesis refuted by its own ladder.
#
# WHAT IS LEFT, AND WHY IT IS THE OBVIOUS ONE IN HINDSIGHT
#   `hero.rs::recipe::GRADE` is `[temperature, post_saturation, contrast]` and
#   Pile A moved it `[0.02, 1.00, 1.30]` -> `[-0.06, 0.80, 1.16]`. Temperature is
#   a WHITE BALANCE on the whole frame: it does not care which light lit a pixel,
#   which is exactly the signature the plate shows (uniformly blue, including
#   faces the window cannot see). No light-rig lever can produce that; a white
#   balance can only produce that.
#
# AND THE SECOND FAILURE, WHICH IS NOT A COLOUR PROBLEM
#   Every rung of ladder #1 FAILED `grade_axes` p95 (192-217 against a 150..185
#   band); the pre-Pile-A frame passed at 177.4. p95 is a luminance axis, so it
#   is the exposure pair (`EXPOSURE` 9.0 -> 8.15, `SHOULDER` 0.64 -> 0.92), not
#   the colour. Fixing one at a time would let each "fix" be judged against the
#   other one's damage, so both move here, on a grid.
#
# THE GRID IS FULL AND EVERY CELL IS REPORTED — per-plate, per-axis, no cell
# dropped, no row collapsed to one number. (2026-08-09: I published a ladder that
# was one plate's numbers wearing the whole set's name. Never again.)
#
# USAGE  scripts/_fl_v4_wbgrid_20260818.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
OUT="$PWD/_fl_v4_20260818/wbgrid"
mkdir -p "$OUT"
EXE="${1:-target-pixel/release/voxelforge_shot.exe}"
[ -f "$EXE" ] || { echo "REFUSED  no exe at $EXE"; exit 2; }
stamp() { date "+[%H:%M:%S]"; }
echo "$(stamp) exe $EXE  $(stat -c %s "$EXE") bytes"

# post_saturation and contrast stay at Pile A's committed 0.80 / 1.16 in every
# cell, so the only things moving are the two axes named on the grid.
TEMPS="-0.06 -0.02 0.02"
EXPOS="8.15 8.55 8.95"

ARGS=""
for t in $TEMPS; do
  for e in $EXPOS; do
    name="t${t}_e${e}"
    png="$OUT/${name}-nohud2.png"
    rm -f "$png"
    VOXELFORGE_GRADE="${t},0.80,1.16" VOXELFORGE_EXPOSURE="$e" \
      VOXELFORGE_SHOT="$png" "./$EXE" > "$OUT/$name.log" 2>&1
    if [ -f "$png" ]; then
      echo "$(stamp) cell temp=$t exposure=$e -> $(stat -c %s "$png") bytes"
      ARGS="$ARGS ${name}=$png"
    else
      echo "$(stamp) cell temp=$t exposure=$e FAILED (see $name.log)"
    fi
  done
done

echo
echo "$(stamp) art-gap instrument, every cell"
python scripts/_fl_v4_grade_20260818.py measure $ARGS --out wbgrid
echo
echo "$(stamp) look-acceptance gate, every cell"
python scripts/grade_axes.py --profile hero "$OUT"/*-nohud2.png 2>&1 \
  | grep -E "wbgrid/|p95|clip|=>"
