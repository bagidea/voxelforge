#!/usr/bin/env bash
# WIDE amber polish (Pixel). we0 passes the GATE but reads too RED/uniform vs the
# ref's softer amber. Pull the orange dye + saturation down for more tonal range
# and an amber (not fire-red) floor, without dropping G3 shade floor below 8%.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1 VOXELFORGE_WIDE=1
export VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
run wp1.png VOXELFORGE_BLUESCALE=0.42 VOXELFORGE_AMBIENT=3400 VOXELFORGE_EXPOSURE=9.0 VOXELFORGE_GRADE=0.04,0.90,1.22
run wp2.png VOXELFORGE_BLUESCALE=0.45 VOXELFORGE_AMBIENT=3700 VOXELFORGE_EXPOSURE=9.1 VOXELFORGE_GRADE=0.05,0.92,1.22
run wp3.png VOXELFORGE_BLUESCALE=0.48 VOXELFORGE_AMBIENT=3200 VOXELFORGE_EXPOSURE=9.2 VOXELFORGE_GRADE=0.03,0.88,1.20
echo ALL_DONE
