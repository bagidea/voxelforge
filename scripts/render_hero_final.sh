#!/usr/bin/env bash
# Final hero at the fp1 framing (window+god-ray left, hero bowl centre, taller
# teal accent, fridge right) with the warmed ambient. Small exposure bracket to
# land near the golden ref frame-mean (~R128,G59,B10). New files — never touches
# tune-v2/v3. Verifies exit code + PNG mtime.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52
COMMON="VOXELFORGE_CAM=$CAM VOXELFORGE_FOG=0.032 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=11,3.2"

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  [[ $? -ne 0 ]] && { echo "FAIL $out"; return; }
  [[ -f "$out" ]] && echo "OK $out ($(stat -c%s "$out")b)" || echo "MISS $out"; }

run hero-golden-a.png $COMMON VOXELFORGE_EXPOSURE=8.4 VOXELFORGE_AMBIENT=2200
run hero-golden-b.png $COMMON VOXELFORGE_EXPOSURE=8.6 VOXELFORGE_AMBIENT=2000
run hero-golden-c.png $COMMON VOXELFORGE_EXPOSURE=8.4 VOXELFORGE_AMBIENT=2600
echo "ALL_DONE"
