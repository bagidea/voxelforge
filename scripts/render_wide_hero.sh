#!/usr/bin/env bash
# CANONICAL WIDE establishing hero shot (Flamingo, 2026-07-26).
# Renders the promoted wide frame -> docs/assets/wide-hero-final.png (tracked;
# the repo .gitignore drops root *.png, so the canonical lives under docs/assets/).
#
# CAMERA (2026-07-27, CEO-approved): TILT-DOWN wide-B — eye (7.6,6.4,-6.0) looking
# DOWN at target (7.6,2.7,8.0), dy 3.7, fov 60. This replaces the old near-level
# wide-A cam (9.0,6.2,-6.5 -> 6.8,2.6,8.0, fov 58) that framed the bowl edge-on so it
# filled the frame without reading AS a bowl. Looking down opens the counter top +
# bowl cavity, which was the whole point of the tilt. Framing was CEO-locked before
# this; the unlock is env-only (hero.rs untouched) and re-measured, not assumed —
# gate + axes readout in the ATMOSPHERE PIN / GATE lines below were re-run on THIS cam.
#
# Look: KEY-LIGHT-DOMINANT rebalance (3/4 toward the window), unchanged by the tilt.
# The earlier wide-A/B/C were AMBIENT-dominated (AMBIENT=4500, sun left at its 12000
# default) -> flat fire-orange, low saturation, high blue: passed the G3/G5/G6 gate
# but failed 5/6 P0 axes and looked nothing like the key-lit golden ref. This recipe
# ports the hero-look-final balance (raise the sun, cut the flat wash) onto the wide
# and deepens DOF so the establishing floor stays crisp voxel geometry (G1).
#
# Result of THIS recipe (geometry pass: de-checkered honey walls + parquet floor +
# teal-glass accent). Measured grade_axes / grade_gate @ 1280x720. Source of truth =
# docs/note-hero-look-final-recipe.md "geometry pass" readout; keep these two in sync.
#   GATE (re-measured on the TILT-DOWN cam, 2026-07-27 — not carried over from wide-A)
#         G3 interior p05-L 14.3% (>=8), darkest (45,22,7) R-B +38 warm
#         G5 brightest (246,205,124) min(G,B)=124 (<=245), 3-pt spread 43.8 (>=8)
#         G6 sunlit wood (232,211,181) R-B +51 L=83.7 (R>G>B)              -> ALL PASS
#   P0    warmth 129 (>=110) · blue 5.2 (<=10) · sat 94 (>=90) · micro 5.92 (>=5)
#         · p95 177 (150..185)                                             -> 5/6 PASS
#         (micro-contrast IMPROVED 5.52 -> 5.92 on the tilt: the down-angle puts more
#          lit counter-top plank edges in frame, which is grain the axis rewards.)
#   ATMOSPHERE PIN (dust motes + LUT grade, 2026-07-27): DUST=3.0 fills the window
#         god-ray corridor with lit warm specks; SHOULDER=0.64 + GRADE contrast 1.30
#         are the LUT half — the shoulder seats the window p95 back in the 150..185
#         band (was 194 over-band without it) while the raised midtone contrast lifts
#         micro-contrast to 5.5 (>=5): the old 4.9 near-miss is now a CLEAN PASS. The
#         motes' hi-freq grain + the contrast lift close the micro axis the smooth
#         walls used to cost, with zero re-checkering of the walls.
#   P0 DOF fg:bg 0.18 (<3): INTRINSIC to a wide establishing frame — that axis wants
#         shallow-DOF-with-textured-foreground (the tight hero composition); a deep-focus
#         wide has fg≈bg sharpness by design. Documented, not a defect.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge_shot.exe
[[ -f "$EXE" ]] || { echo "NO EXE $EXE — run: cargo build --release --bin voxelforge_shot"; exit 2; }
OUT=docs/assets/wide-hero-final.png
env \
  VOXELFORGE_WIDE=1 \
  VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60 \
  VOXELFORGE_DOF=8,10 \
  VOXELFORGE_SUN=19,196,26000 \
  VOXELFORGE_AMBIENT=2800 \
  VOXELFORGE_BLUESCALE=0.85 \
  VOXELFORGE_EXPOSURE=9.0 \
  VOXELFORGE_GRADE=0.02,1.00,1.30 \
  VOXELFORGE_SHOULDER=0.64 \
  VOXELFORGE_DUST=3.0 \
  VOXELFORGE_BOUNCE=1.0 \
  VOXELFORGE_BOUNCE2=1.7 \
  VOXELFORGE_AMBCOLOR=0.70,0.60,0.44 \
  VOXELFORGE_SHOT="$OUT" "$EXE" >"logs_wide_hero.txt" 2>&1
code=$?
[[ $code -ne 0 ]] && { echo "FAIL exit=$code (see logs_wide_hero.txt)"; exit 1; }
[[ -f "$OUT" ]] && echo "OK   $OUT ($(stat -c%s "$OUT")b)" || { echo "MISS $OUT"; exit 1; }
