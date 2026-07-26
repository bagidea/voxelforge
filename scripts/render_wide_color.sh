#!/usr/bin/env bash
# WIDE color pullback (Pixel, 2026-07-26). The wide establishing framing (w-c,
# 3/4 toward the window) reads great in GEOMETRY but collapses into monochrome
# RED: the deepened enclosed room + baked warm settings (BLUESCALE 0.50, AMBIENT
# 4200, GRADE 0.10/1.02/1.30, EXPOSURE 9.5) over-dye every surface. Same failure
# the non-wide pd-baked frame hit, but stronger. Levers (all env, no recompile):
#   BLUESCALE ↑  let blue back so it's not one-hue red (steel/green/cream survive)
#   AMBIENT   ↓  less honey dye across the big lit floor/walls
#   EXPOSURE  ↑  darkens → pulls highlights out of the red clip (ev100 up = dimmer)
#   GRADE temp↓ sat↓  stop the post-grade shoving midtones redder
# Camera locked to w-c. Verifies exit + output per frame.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
export VOXELFORGE_WIDE=1
export VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58
export VOXELFORGE_DOF=12.5,1.6

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# c1: gentle pullback
run wc1.png VOXELFORGE_BLUESCALE=0.75 VOXELFORGE_AMBIENT=3200 VOXELFORGE_EXPOSURE=10.0 VOXELFORGE_GRADE=0.05,0.98,1.22
# c2: stronger — more blue, lower ambient, dimmer
run wc2.png VOXELFORGE_BLUESCALE=0.90 VOXELFORGE_AMBIENT=2800 VOXELFORGE_EXPOSURE=10.3 VOXELFORGE_GRADE=0.02,0.94,1.20
# c3: strongest pullback
run wc3.png VOXELFORGE_BLUESCALE=1.05 VOXELFORGE_AMBIENT=2500 VOXELFORGE_EXPOSURE=10.6 VOXELFORGE_GRADE=0.00,0.92,1.18
# c4: mid, warmer-held (keep some golden, just kill the red clip via exposure)
run wc4.png VOXELFORGE_BLUESCALE=0.82 VOXELFORGE_AMBIENT=3000 VOXELFORGE_EXPOSURE=10.4 VOXELFORGE_GRADE=0.04,0.96,1.24
echo "ALL_DONE"
