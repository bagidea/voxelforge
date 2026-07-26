#!/usr/bin/env bash
# Render one hero variant with the isolated voxelforge_shot exe, then grade it.
# Usage: render_grade.sh <out.png> [env assignments...]
#   e.g. render_grade.sh final-default.png
#        render_grade.sh final-camB.png VOXELFORGE_CAM=6.5,3.9,-3.0,8.5,2.6,8.0,50
set -u
cd "$(dirname "$0")/.."
OUT="$1"; shift || true
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
