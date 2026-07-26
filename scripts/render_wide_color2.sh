#!/usr/bin/env bash
# WIDE color pullback round 2 (Pixel, 2026-07-26). Round-1 diagnosis was wrong-way:
# I raised ev100 (darkened) — p95 crashed to ~55-73. The grader shows baked-wide
# actually PASSES warmth/blue/sat and only FAILS p95 (too dark, 123-138 vs 150-185).
# The frame reads "red" not because it's over-warm but because it's FIRE-red (high R
# chroma, low luminance) vs the ref's AMBER (G substantial). The right levers:
#   • lower post_saturation + neutral temperature  → brings GREEN up toward R
#     (amber, not red) AND raises luminance (G carries 0.71 luma weight) → p95 up.
#   • lower ev100 (brighter) → lifts window/sunlit p95 into the 150-185 band.
#   • modest bluescale (blue axis has huge margin at 1.0; keep warmth >=110).
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

# d1: brighten + de-red via sat/temp, hold ambient
run wd1.png VOXELFORGE_BLUESCALE=0.65 VOXELFORGE_AMBIENT=3800 VOXELFORGE_EXPOSURE=9.1 VOXELFORGE_GRADE=0.02,0.94,1.24
# d2: brighter still + a touch more de-sat
run wd2.png VOXELFORGE_BLUESCALE=0.65 VOXELFORGE_AMBIENT=3800 VOXELFORGE_EXPOSURE=8.9 VOXELFORGE_GRADE=0.00,0.92,1.22
# d3: hold warmth higher (less blue), brighten hard
run wd3.png VOXELFORGE_BLUESCALE=0.55 VOXELFORGE_AMBIENT=4000 VOXELFORGE_EXPOSURE=8.9 VOXELFORGE_GRADE=0.03,0.95,1.24
# d4: strongest brighten, neutral grade
run wd4.png VOXELFORGE_BLUESCALE=0.60 VOXELFORGE_AMBIENT=3900 VOXELFORGE_EXPOSURE=8.7 VOXELFORGE_GRADE=0.00,0.93,1.22
echo "ALL_DONE"
