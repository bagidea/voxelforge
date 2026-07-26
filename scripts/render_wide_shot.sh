#!/usr/bin/env bash
# WIDE compare render via the CLEAN isolated binary `voxelforge_shot` (Flamingo,
# 2026-07-26). Same amber-locked color as render_wide_finals.sh, but driven through
# shot_main.rs (#[path]-includes hero.rs → renders setup_hero ONLY, no main.rs).
# voxelforge_shot does NOT need VOXELFORGE_HERO — it always runs the hero scene.
# AMBER color = wag1 (G leg 0.541->0.630 + AMBIENT 4500) → holds the G3 shade floor
# and lifts the wide floor/wall out of fire-red into the ref's amber wood.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge_shot.exe
[[ -f "$EXE" ]] || { echo "NO EXE $EXE"; exit 2; }
export VOXELFORGE_WIDE=1
# HONEY GRADE (Flamingo, tone4 — matched to hero-golden-b.png by measured tone):
# the amber-locked color (bluescale 0.42 / exposure 9.0 / sat 0.95) rendered the wide
# room FIRE-RED + neon: measured floor/wall meanRGB (136,60,17) R-G=76, sat p90 0.94,
# 45% "fire" pixels vs the golden-b ref's soft honey (117,61,27) R-G=56, sat 0.89, 12%.
# The fix is the GRADE, not the albedo (albedo is already warm walnut). Levers:
#   • BLUESCALE 0.42->0.67  — lifts the crushed blue leg (G-B 44->42, un-crushes shade)
#   • EXPOSURE  9.0 ->8.75  — pulls the blown reds down off clipping
#   • GRADE sat 0.95->0.88 + temp 0.03->-0.01 — post_saturation is what RE-created the
#     fire-red after tonemap; dropping it (and cooling temp) is the dominant desat lever
#   • AMBCOLOR (0.784,0.630,0.200)->(0.75,0.605,0.30) — cooler amber, blue leg lifted so
#     the ambient-dominated walls read honey-tan, not orange
# Result on wide-A: meanRGB (126,72,31) R-G=54 (ref 56), sat p90 0.86, dark floor tiles
# read warm BROWN not black, terracotta island like the ref. G3 p05=15.6% (passes with
# headroom), G5/G6 still PASS. contrast stays 1.10 (holds the G3 shade floor).
export VOXELFORGE_BLUESCALE=0.67 VOXELFORGE_EXPOSURE=8.75 VOXELFORGE_GRADE=-0.01,0.88,1.10
export VOXELFORGE_AMBCOLOR=0.75,0.605,0.30 VOXELFORGE_AMBIENT=4500
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
