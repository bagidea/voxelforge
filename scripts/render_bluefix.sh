#!/usr/bin/env bash
# P0-BLUE sweep: close the grade_axes gap toward REF WITHOUT touching framing.
# Levers = VOXELFORGE_BLUESCALE (multiplies the B leg of sun+ambient+fog) x exposure.
# Default cam is left untouched (CEO owns the wide staged framing separately).
# Renders each variant with the real hero exe, then grades all 6 axes + G3/G5/G6.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
[ -x "$EXE" ] || { echo "NO EXE: $EXE"; exit 2; }

run() {
  local out="$1"; shift
  rm -f "$out"
  env VOXELFORGE_HERO=1 "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 || ! -f "$out" ]]; then echo "FAIL $out exit=$code"; return; fi
  echo "OK   $out"
}

# bscale x exposure grid. bg-melt via wide aperture (focus stays on the bowl=10).
run pf-b45-e100.png VOXELFORGE_BLUESCALE=0.45 VOXELFORGE_EXPOSURE=10.0 VOXELFORGE_DOF=10,1.4
run pf-b55-e100.png VOXELFORGE_BLUESCALE=0.55 VOXELFORGE_EXPOSURE=10.0 VOXELFORGE_DOF=10,1.4
run pf-b55-e102.png VOXELFORGE_BLUESCALE=0.55 VOXELFORGE_EXPOSURE=10.2 VOXELFORGE_DOF=10,1.4
run pf-b65-e102.png VOXELFORGE_BLUESCALE=0.65 VOXELFORGE_EXPOSURE=10.2 VOXELFORGE_DOF=10,1.4

echo "==== AXES ===="
python scripts/grade_axes.py pf-b45-e100.png pf-b55-e100.png pf-b55-e102.png pf-b65-e102.png
echo "ALL_DONE"
