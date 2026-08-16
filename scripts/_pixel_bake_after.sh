#!/usr/bin/env bash
# _pixel_bake_after.sh -- AFTER plates for ship-blocker #3 (pixel lane, 2026-08-16).
#
# The claim being tested: hero.rs's baked `mod recipe` reproduces the CEO-approved
# docs/assets/wide-hero-final.png with NO env at all.
#
# Plates, in the order that makes the claim falsifiable:
#   default-bare    -- the whole point. Only VOXELFORGE_SHOT is set (an output path,
#                      not a look knob). Any other env here would launder the bake.
#   default-bare-r2 -- same binary, same (empty) env, shot again. This is the CAPTURE
#                      NOISE FLOOR: without it a diff of N pixels means nothing,
#                      because I do not yet know what 0 means on this box.
#   recipe-env      -- the OLD path: every recipe var exported explicitly. If the bake
#                      is faithful this must equal default-bare to within the floor.
#                      If it does NOT, I transcribed a constant wrong.
#   narrow-cam      -- baked look + the retired narrow camera via VOXELFORGE_CAM. Keeps
#                      the tight-framing diagnostic alive now that the narrow scene is
#                      no longer the default (see mod recipe).
set -u
cd "$(dirname "$0")/.."

EXE="${EXE:-target-pixel/release/voxelforge_shot.exe}"
OUT="${OUT:-_fl_bake_20260816/after}"
[[ -f "$EXE" ]] || { echo "NO EXE $EXE -- run: .\\scripts\\lane-build.ps1 -Lane pixel -Profile release -Bin voxelforge_shot"; exit 2; }
mkdir -p "$OUT"

echo "EXE   $EXE"
echo "size  $(stat -c%s "$EXE")b"
echo "mtime $(stat -c%y "$EXE")"
echo "md5   $(md5sum "$EXE" | cut -d' ' -f1)"
echo

shoot() {
  local name="$1"; shift
  local out="$OUT/$name-nohud2.png"
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" > "$OUT/$name.runlog" 2>&1
  local code=$?
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b) exit=$code"
  else echo "MISS $out exit=$code"; return 1; fi
}

# NO env. Not one knob. This plate is the deliverable.
shoot default-bare
shoot default-bare-r2

shoot recipe-env \
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
  VOXELFORGE_AMBCOLOR=0.70,0.60,0.44

shoot narrow-cam VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52 VOXELFORGE_DOF=10,1.4

echo DONE
