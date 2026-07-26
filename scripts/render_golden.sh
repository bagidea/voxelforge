#!/usr/bin/env bash
# Golden-match sweep: bright luminous golden-hour like docs/assets/golden-beauty-shot-ref.png
# (frame-mean target ~R128,G59,B10; G3 shadow floor >=8 warm; teal accent visible).
# tune-v2/v3 were too DARK (R79) -> read as muddy orange. Push exposure + ambient up,
# reframe so the hero bowl fills the foreground and the teal block reads.
# Env-driven, no recompile. Verifies exit code + PNG mtime advanced.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

run() {
  local out="$1"; shift
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return; fi
  if [[ -f "$out" ]]; then echo "OK   $out  ($(stat -c%s "$out") bytes)";
  else echo "MISS $out — no png (see logs_$out.txt)"; fi
}

# CAM A: eye at table height, closer, gentle look-down onto the island (golden framing)
CAMA=8.2,4.5,-2.2,7.3,2.7,6.0,50
# CAM B: a touch higher + wider so more warm table fills the lower frame
CAMB=8.0,4.9,-3.2,7.4,2.5,6.2,52

run gm-a1.png VOXELFORGE_CAM=$CAMA VOXELFORGE_EXPOSURE=8.0 VOXELFORGE_AMBIENT=2400 \
  VOXELFORGE_FOG=0.03 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=6.2,2.8
run gm-a2.png VOXELFORGE_CAM=$CAMA VOXELFORGE_EXPOSURE=7.7 VOXELFORGE_AMBIENT=2800 \
  VOXELFORGE_FOG=0.03 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=6.2,2.8
run gm-b1.png VOXELFORGE_CAM=$CAMB VOXELFORGE_EXPOSURE=8.0 VOXELFORGE_AMBIENT=2400 \
  VOXELFORGE_FOG=0.03 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=8.0,2.8
run gm-b2.png VOXELFORGE_CAM=$CAMB VOXELFORGE_EXPOSURE=7.7 VOXELFORGE_AMBIENT=2800 \
  VOXELFORGE_FOG=0.028 VOXELFORGE_DFOG=0.007 VOXELFORGE_DOF=8.0,2.8

echo "ALL_DONE"
