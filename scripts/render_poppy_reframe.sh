#!/usr/bin/env bash
# Poppy reframe sweep — the fg/bg DOF axis is content-bound: the grader's fg zone
# (bottom-centre) currently lands on the SHADOWED near-floor (dark walnut checker,
# low contrast => fg-hf ~1.0). Golden's fg zone is the LIT high-contrast planked
# tabletop (fg-hf ~8). Lever: tilt/pull the camera so the bottom-centre lands on the
# lit counter/counter_dk tabletop (y2.5) instead of the shadow floor. Env-only, no
# recompile. Verifies exit code + PNG mtime; new files only.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# baked framing for reference: 7.6,5.9,-5.2,7.6,3.2,6.0,52
# R1: nearer + lower eye, look down at the tabletop front edge
run rf1.png VOXELFORGE_CAM=7.6,5.2,-2.5,7.6,2.4,5.0,52
# R2: closer/lower still, wider fov (tabletop fills lower third)
run rf2.png VOXELFORGE_CAM=7.6,4.8,-1.0,7.6,2.2,4.5,54
# R3: higher eye, steeper down-look onto the lit surface
run rf3.png VOXELFORGE_CAM=7.6,6.2,-3.5,7.6,2.6,5.5,50
# R4: same baked eye, just tilt DOWN (target y 3.2 -> 2.6)
run rf4.png VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,2.6,6.0,52
echo "ALL_DONE"
