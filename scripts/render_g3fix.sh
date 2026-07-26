#!/usr/bin/env bash
# G3 SURGICAL FIX — hold tune-v2's EXACT frame (cam/exposure/fog/dof), lift ONLY the
# warm bounce fill (VOXELFORGE_AMBIENT) so the shade floor clears G3 (interior p05-L>=8%,
# target ~ref 9.9%). CEO locked the tune-v2 framing: do NOT reframe. One variable moved.
# Verifies exit code + that the PNG actually (re)appeared so a crash can't fake a good frame.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

# tune-v2's exact frame params (from render_variants.sh v2) — held constant:
CAM=7.0,5.3,-4.5,7.5,2.4,6.0,52
EXP=9.0
FOG=0.035
DOF=8.5,2.4

run() {
  local out="$1"; shift
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return; fi
  if [[ -f "$out" ]]; then echo "OK   $out  ($(stat -c%s "$out") bytes)";
  else echo "MISS $out — no png (see logs_$out.txt)"; fi
}

# Pure ambient ladder (only knob that moves) — brackets 6.0% -> ~8..12% p05.
run g3-a2000.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=$EXP VOXELFORGE_FOG=$FOG VOXELFORGE_DOF=$DOF VOXELFORGE_AMBIENT=2000
run g3-a2600.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=$EXP VOXELFORGE_FOG=$FOG VOXELFORGE_DOF=$DOF VOXELFORGE_AMBIENT=2600
run g3-a3200.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=$EXP VOXELFORGE_FOG=$FOG VOXELFORGE_DOF=$DOF VOXELFORGE_AMBIENT=3200
# One hedge: modest ambient + a touch more exposure (lower ev100) to also lift midtone
# toward ref p50=23.8 WITHOUT over-flattening GI — compare against pure-ambient family.
run g3-e86a2200.png VOXELFORGE_CAM=$CAM VOXELFORGE_EXPOSURE=8.6 VOXELFORGE_FOG=$FOG VOXELFORGE_DOF=$DOF VOXELFORGE_AMBIENT=2200

echo "ALL_DONE"
