#!/usr/bin/env bash
# WIDE amber G3-recovery (Pixel). wam2 amber hue reads great but G3 p05 fell to 7.3
# (need >=8) — brighter lit surfaces widened the histogram. Lift the shade floor
# back with more ambient POWER at the amber hue; watch p95 stays <=185.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1 VOXELFORGE_WIDE=1
export VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
export VOXELFORGE_BLUESCALE=0.42 VOXELFORGE_EXPOSURE=9.0 VOXELFORGE_GRADE=0.03,0.95,1.22
run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
run wag1.png VOXELFORGE_AMBCOLOR=0.784,0.630,0.200 VOXELFORGE_AMBIENT=4500
run wag2.png VOXELFORGE_AMBCOLOR=0.784,0.650,0.205 VOXELFORGE_AMBIENT=4800
run wag3.png VOXELFORGE_AMBCOLOR=0.784,0.620,0.195 VOXELFORGE_AMBIENT=5000
run wag4.png VOXELFORGE_AMBCOLOR=0.784,0.640,0.200 VOXELFORGE_AMBIENT=5300
echo ALL_DONE
