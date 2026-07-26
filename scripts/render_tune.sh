#!/usr/bin/env bash
# Hero-frame tuning sweep (env-driven, no recompile). Each run auto-screenshots
# at 3.2s and exits at 4.4s. We check the exit code AND that the PNG mtime
# advanced, so a crashed run can never masquerade as a good frame.
# Reframe: camera raised, looking DOWN onto the island so warm floor fills the
# lower frame (ref frame-mean R≈128, ours was 79 = underexposed). Lower ev100 +
# higher ambient lifts the shadow floor to the ref's p05≈10%.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

run() {
  local out="$1"; shift
  rm -f "$out"
  local before=0; [[ -f "$out" ]] && before=$(stat -c%Y "$out")
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return; fi
  if [[ -f "$out" ]]; then
    echo "OK   $out  ($(stat -c%s "$out") bytes)"
  else
    echo "MISS $out — no png (see logs_$out.txt)"
  fi
}

CAM=7.4,6.1,-4.8,7.7,2.1,6.5,50

# a: moderate brighten
run htA.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=8.6 VOXELFORGE_AMBIENT=1700 \
  VOXELFORGE_FOG=0.035 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=9.4,2.8
# b: brighter + more ambient (push G3 floor up)
run htB.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=8.3 VOXELFORGE_AMBIENT=2100 \
  VOXELFORGE_FOG=0.035 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=9.4,2.8
# c: brightest interior, sun a touch higher for softer key
run htC.png VOXELFORGE_CAM=$CAM VOXELFORGE_SUN=24,198,12000 VOXELFORGE_EXPOSURE=8.0 \
  VOXELFORGE_AMBIENT=2500 VOXELFORGE_FOG=0.03 VOXELFORGE_DFOG=0.007 VOXELFORGE_DOF=9.4,2.8

echo "ALL_DONE"
