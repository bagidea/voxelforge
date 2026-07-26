#!/usr/bin/env bash
# WIDE final tuning (Pixel, 2026-07-26). Two axes left after round 2:
#   • blue 13.8 (need <=10) — brightening lifted it → cut bluescale 0.55→0.42.
#   • DOF fg:bg 0.57 (need >=3.0) — the bowl sits MID-distance so the near floor is
#     blurrier than the far wall (ratio <1). The ref fixes this by putting the bowl
#     CLOSE in the foreground with focus ON it and the deep room melting behind.
#     So: pull the camera IN toward the bowl (eye z -1..-2), lower it, wide FOV, and
#     focus onto the bowl plane (~5-6m) with a tighter aperture so bg melts.
# Color base: bluescale 0.42, ambient 4000, exposure 9.0, grade 0.03,0.95,1.22.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
export VOXELFORGE_WIDE=1
export VOXELFORGE_BLUESCALE=0.42
export VOXELFORGE_AMBIENT=4000
export VOXELFORGE_EXPOSURE=9.0
export VOXELFORGE_GRADE=0.03,0.95,1.22

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# e0: w-c framing unchanged, just blue-fixed (baseline to confirm 5/6, DOF still low)
run we0.png VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
# e1: bowl-forward 3/4 toward window, focus on bowl ~5.8, tighter ap 2.0
run we1.png VOXELFORGE_CAM=8.6,5.2,-1.5,6.9,2.5,9.0,62 VOXELFORGE_DOF=5.8,2.0
# e2: same, focus a touch back + wider ap for softer bg
run we2.png VOXELFORGE_CAM=8.6,5.2,-1.5,6.9,2.5,9.0,62 VOXELFORGE_DOF=6.5,1.6
# e3: bowl-forward straight-on (bowl centered), focus 5.5
run we3.png VOXELFORGE_CAM=7.6,5.0,-1.0,7.6,2.4,9.0,64 VOXELFORGE_DOF=5.5,1.8
# e4: lower + closer, bowl big in foreground, focus 4.8, tight ap
run we4.png VOXELFORGE_CAM=8.4,4.6,-0.5,6.9,2.3,9.0,64 VOXELFORGE_DOF=4.8,2.2
echo "ALL_DONE"
