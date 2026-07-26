#!/usr/bin/env bash
# Framing probe: find a camera that reads like the golden ref — hero bowl on the
# island top, window + god-rays screen-left, fridge screen-right, room depth.
# Fixed moderate-bright exposure so we judge COMPOSITION not brightness.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
COMMON="VOXELFORGE_EXPOSURE=8.4 VOXELFORGE_AMBIENT=2200 VOXELFORGE_FOG=0.032 VOXELFORGE_DFOG=0.008"

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  [[ $? -ne 0 ]] && { echo "FAIL $out"; return; }
  [[ -f "$out" ]] && echo "OK $out ($(stat -c%s "$out")b)" || echo "MISS $out"; }

# f1: back + up, aim at island TOP (ty raised to 3.2 so the bowl sits on a visible surface)
run fp1.png $COMMON VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52 VOXELFORGE_DOF=11,3.2
# f2: a bit closer, over-the-counter onto the bowl, window stays left
run fp2.png $COMMON VOXELFORGE_CAM=8.4,5.2,-4.0,7.0,3.0,5.8,52 VOXELFORGE_DOF=9,2.8
# f3: higher + wider establishing shot, more room + floor + god-ray band
run fp3.png $COMMON VOXELFORGE_CAM=7.5,6.6,-6.0,7.8,2.9,6.5,56 VOXELFORGE_DOF=12,3.5
echo "ALL_DONE"
