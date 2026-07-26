#!/usr/bin/env bash
# WIDE amber-hue sweep (Pixel). The wide frame collapses to FIRE-RED because the
# ambient dye's G leg (0.541) is too low vs the ref's amber. Lift G via the new
# VOXELFORGE_AMBCOLOR env (shipped default byte-identical when unset). Rest = we0
# base. w-c framing. Verifies exit + output per frame.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1 VOXELFORGE_WIDE=1
export VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
export VOXELFORGE_BLUESCALE=0.42 VOXELFORGE_AMBIENT=4000 VOXELFORGE_EXPOSURE=9.0
export VOXELFORGE_GRADE=0.03,0.95,1.22
run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
run wam0.png VOXELFORGE_AMBCOLOR=0.784,0.541,0.180   # control = shipped hue
run wam1.png VOXELFORGE_AMBCOLOR=0.784,0.600,0.190
run wam2.png VOXELFORGE_AMBCOLOR=0.784,0.650,0.205
run wam3.png VOXELFORGE_AMBCOLOR=0.760,0.680,0.220
run wam4.png VOXELFORGE_AMBCOLOR=0.740,0.700,0.235
echo ALL_DONE
