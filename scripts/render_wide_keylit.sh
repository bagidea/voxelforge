#!/usr/bin/env bash
# WIDE establishing shot — KEY-LIGHT-DOMINANT rebalance, ROUND 3 (Flamingo, 2026-07-26).
# R1 (wk*): grade fixes nail blue/sat/micro but ev too high -> p95+G3 crashed.
# R2 (wm*): 5/6 P0 axes PASS (warmth/blue/sat/micro/p95) but G3 p05 fell to ~7%
#   (need >=8): contrast 1.18-1.22 + high grade-sat crushed deep-shade G/B to 0
#   (darkest px (70,0,0)) -> luminance floor too low.
# R3: LIFT the shadow floor while holding the rest. Warmth/sat/p95 carry huge margin,
#   so: ease contrast (1.18->~1.13), lift AMBIENT + its GREEN leg (G has 0.71 luma
#   weight, so it lifts p05 fast and stays warm), ease grade-sat off the channel-crush,
#   lift bluescale a touch (blue axis has margin to spare). SUN = elev,azim,illuminance.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge_shot.exe
[[ -f "$EXE" ]] || { echo "NO EXE $EXE"; exit 2; }
CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58   # wide-A: 3/4 toward the window (ref angle)
DOF=12.5,1.6
run() { local out="$1"; shift; rm -f "$out"
  env VOXELFORGE_WIDE=1 VOXELFORGE_CAM="$CAM" VOXELFORGE_DOF="$DOF" "$@" \
    VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
# wn1: balanced lift
run wn1.png VOXELFORGE_SUN=18,196,18000 VOXELFORGE_AMBIENT=4500 \
    VOXELFORGE_BLUESCALE=0.58 VOXELFORGE_EXPOSURE=8.80 VOXELFORGE_GRADE=0.02,0.99,1.14 \
    VOXELFORGE_AMBCOLOR=0.78,0.63,0.28
# wn2: a touch more key + contrast (micro safety), floor lifted via ambient G leg
run wn2.png VOXELFORGE_SUN=18,196,19000 VOXELFORGE_AMBIENT=4400 \
    VOXELFORGE_BLUESCALE=0.57 VOXELFORGE_EXPOSURE=8.85 VOXELFORGE_GRADE=0.02,1.01,1.16 \
    VOXELFORGE_AMBCOLOR=0.78,0.62,0.27
# wn3: safest floor (most ambient, lowest contrast/sat)
run wn3.png VOXELFORGE_SUN=17,196,17000 VOXELFORGE_AMBIENT=4700 \
    VOXELFORGE_BLUESCALE=0.60 VOXELFORGE_EXPOSURE=8.72 VOXELFORGE_GRADE=0.01,0.97,1.12 \
    VOXELFORGE_AMBCOLOR=0.79,0.64,0.30
# wn4: key-forward but floor-safe (higher sun, ambient still generous)
run wn4.png VOXELFORGE_SUN=19,196,20000 VOXELFORGE_AMBIENT=4400 \
    VOXELFORGE_BLUESCALE=0.57 VOXELFORGE_EXPOSURE=8.82 VOXELFORGE_GRADE=0.02,1.00,1.15 \
    VOXELFORGE_AMBCOLOR=0.78,0.63,0.28
echo ALL_DONE
