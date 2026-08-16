#!/usr/bin/env bash
# _pixel_bake_probe.sh -- EXPLORATORY probe for ship-blocker #3 (pixel lane).
#
# Question: is the red-clipped default plate caused by the LOOK knobs alone, or
# does it also need the WIDE framing? Answer it with env BEFORE editing hero.rs,
# so the bake I commit is the smallest one that works.
#
#   p-lookonly-tight : recipe look knobs, DEFAULT (tight) framing + default DOF
#   p-lookdof-tight  : same + the recipe's deep-focus DOF
#   p-full-tight     : same + WIDE geometry but the DEFAULT tight camera
#
# Not a proof of anything shippable -- these all use env crutches on purpose.
set -u
cd "$(dirname "$0")/.."

EXE="${EXE:-target-pixel/release/voxelforge_shot.exe}"
OUT="${OUT:-_fl_bake_20260816/probe}"
[[ -f "$EXE" ]] || { echo "NO EXE $EXE"; exit 2; }
mkdir -p "$OUT"

LOOK=(
  VOXELFORGE_SUN=19,196,26000
  VOXELFORGE_AMBIENT=2800
  VOXELFORGE_BLUESCALE=0.85
  VOXELFORGE_EXPOSURE=9.0
  VOXELFORGE_GRADE=0.02,1.00,1.30
  VOXELFORGE_SHOULDER=0.64
  VOXELFORGE_DUST=3.0
  VOXELFORGE_BOUNCE=1.0
  VOXELFORGE_BOUNCE2=1.7
  VOXELFORGE_AMBCOLOR=0.70,0.60,0.44
)

shoot() {
  local name="$1"; shift
  local out="$OUT/$name-nohud2.png"
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" > "$OUT/$name.runlog" 2>&1
  local code=$?
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b) exit=$code"
  else echo "MISS $out exit=$code"; fi
}

shoot p-lookonly-tight "${LOOK[@]}"
shoot p-lookdof-tight  "${LOOK[@]}" VOXELFORGE_DOF=8,10
shoot p-full-tight     "${LOOK[@]}" VOXELFORGE_DOF=8,10 VOXELFORGE_WIDE=1

echo DONE
