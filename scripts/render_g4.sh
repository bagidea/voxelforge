#!/usr/bin/env bash
# Post-fix hero render: PCSS soft shadows (G4a) + Ultra SSAO contact AO (G4b) +
# wide-aperture DOF (bg bokeh) + TAA-cleaned. Brackets framing / soft-size / DOF so
# we can pick the frame that best matches docs/assets/golden-beauty-shot-ref.png.
# New files (g4-*.png) — never touches tune-v2/v3 or hero-golden-*. Verifies exit
# code + PNG mtime (scar: pipe-masked build/render fail).
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

# Framing pulled back + across so the bowl sits ~lower-third and the room reads
# behind it (blurred), like the ref — not the old bowl-fills-frame crop.
CAM=9.2,4.9,-6.2,7.2,2.7,7.5,55
# Focus on the hero bowl (~10m from eye), wide aperture => far cabinets melt.
COMMON="VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=8.6 VOXELFORGE_AMBIENT=2400 VOXELFORGE_DFOG=0.008"

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code (see logs_$out.txt)"; return; fi
  if [[ -f "$out" ]]; then echo "OK   $out  ($(stat -c%s "$out")b, mtime $(stat -c%y "$out" | cut -d. -f1))";
  else echo "MISS $out"; fi; }

# soft-size sweep at the golden framing (penumbra width vs noise trade)
run g4-soft2.png  $COMMON VOXELFORGE_SOFT=2.0 VOXELFORGE_FOG=0.05 VOXELFORGE_DOF=10.0,1.8
run g4-soft4.png  $COMMON VOXELFORGE_SOFT=4.0 VOXELFORGE_FOG=0.05 VOXELFORGE_DOF=10.0,1.8
run g4-soft6.png  $COMMON VOXELFORGE_SOFT=6.0 VOXELFORGE_FOG=0.05 VOXELFORGE_DOF=10.0,1.8
# stronger god-ray fog + slightly wider aperture variant
run g4-ray.png    $COMMON VOXELFORGE_SOFT=4.0 VOXELFORGE_FOG=0.075 VOXELFORGE_DOF=10.0,1.6
echo "ALL_DONE"
