#!/usr/bin/env bash
# Render one hero variant with the isolated voxelforge_shot exe, then grade it.
# Usage: render_grade.sh <out-nohud2.png> [env assignments...]
#   e.g. render_grade.sh final-default-nohud2.png
#        render_grade.sh final-camB-nohud2.png VOXELFORGE_CAM=6.5,3.9,-3.0,8.5,2.6,8.0,50
#
# WHY THE OUTPUT MUST BE NAMED `-nohud2.png`
#   The graders below (scripts/nohud2_guard.py) refuse any frame that is not
#   named de-HUDded, because HUD glyphs at ~250 false-FAIL G5 and drag p95 to the
#   UI. This script renders with `voxelforge_shot.exe` (client/src/shot_main.rs),
#   which draws `hero.rs` and NOTHING else — there is no HUD in these pixels, so
#   the frame is de-HUDded by construction and simply has to say so. This is a
#   naming requirement, not an exemption: a `--play` capture still has to go
#   through scripts/_flamingo_dehud2.py.
set -u
cd "$(dirname "$0")/.."
OUT="$1"; shift || true
case "$OUT" in
  *-nohud2.png) ;;
  *) echo "REFUSED  output must be named <name>-nohud2.png (got: $OUT)"
     echo "         voxelforge_shot renders no HUD, so its frame IS de-HUDded — name it so the"
     echo "         graders can tell. See the header of this script and scripts/nohud2_guard.py."
     exit 2;;
esac
EXE="target/release/voxelforge_shot.exe"
[ -x "$EXE" ] || { echo "NO EXE: $EXE"; exit 2; }
# Apply any KEY=VAL env passed as args.
for kv in "$@"; do export "$kv"; done
export VOXELFORGE_SHOT="$OUT"
rm -f "$OUT"
"./$EXE" > "logs_${OUT}.txt" 2>&1
echo "---- gate ($OUT) ----"
python scripts/grade_gate.py "$OUT" 2>&1 | tail -16
echo "---- G3 p05-L ($OUT) ----"
python scripts/grade_g3.py "$OUT" 2>&1 | grep -iE "p05-L|darkest repres|RGB=|warm"
