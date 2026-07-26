#!/usr/bin/env bash
# WIDE final comparison angles — AMBER color locked (Pixel, 2026-07-26).
# Color = wag1: passes the GATE (G3 p05 8.1 / G5 / G6) with the ambient hue lifted
# toward the ref's amber (VOXELFORGE_AMBCOLOR G leg 0.541->0.630 + AMBIENT 4500 to
# hold the G3 shade floor). Three tilt-down establishing angles for CEO to choose.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1 VOXELFORGE_WIDE=1
export VOXELFORGE_BLUESCALE=0.42 VOXELFORGE_EXPOSURE=9.0 VOXELFORGE_GRADE=0.03,0.95,1.22
export VOXELFORGE_AMBCOLOR=0.784,0.630,0.200 VOXELFORGE_AMBIENT=4500
run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
# A: 3/4 toward the window (ref-matching angle) — PRIMARY
run wide-A.png VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
# B: straight-on wide establishing
run wide-B.png VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60 VOXELFORGE_DOF=12.0,1.6
# C: 3/4 toward the fridge (mirror of A)
run wide-C.png VOXELFORGE_CAM=6.2,6.2,-6.5,8.4,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
echo ALL_DONE
