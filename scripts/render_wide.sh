#!/usr/bin/env bash
# WIDE establishing-shot camera sweep (Pixel, 2026-07-26).
# Geometry: VOXELFORGE_WIDE=1 dresses the deepened room (deep foreground floor,
# taller matte walls, long tabletop, NO hot ceiling). Here we sweep the TILT-DOWN
# camera to find the golden-ref establishing framing that reads the bowl on a
# counter with real depth and NO void. Env-only — no recompile between frames.
# Verifies exit code + output existence per frame (pipe-mask scar).
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
export VOXELFORGE_WIDE=1

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# CAM = eyeX,eyeY,eyeZ, targetX,targetY,targetZ, fovDeg
# Shipped cramped cam for reference: 7.6,5.9,-5.2, 7.6,3.2,6.0, 52
# All below pull back (−Z), raise the eye, and tilt DOWN toward the counter,
# widening FOV so the room reads as an establishing shot. DOF focus pushed onto
# the bowl plane (~eye→bowl depth) so the hero stays sharp while bg melts.

# w-a: moderate pull-back, gentle tilt, fov60
run w-a.png VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60 VOXELFORGE_DOF=12.0,1.6
# w-b: higher + more tilt-down, fov62 (stronger "looking down into the bowl")
run w-b.png VOXELFORGE_CAM=7.6,7.4,-7.5,7.6,2.2,8.5,62 VOXELFORGE_DOF=13.0,1.6
# w-c: 3/4 offset toward the window side (like the ref's angle), fov58
run w-c.png VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
# w-d: 3/4 offset toward the fridge side, fov58
run w-d.png VOXELFORGE_CAM=6.2,6.2,-6.5,8.4,2.6,8.0,58 VOXELFORGE_DOF=12.5,1.6
# w-e: lower + closer eye, bowl bigger in foreground, fov64
run w-e.png VOXELFORGE_CAM=7.6,5.6,-4.5,7.6,2.6,8.5,64 VOXELFORGE_DOF=10.5,1.6
# w-f: deep pull-back for max establishing depth, fov60
run w-f.png VOXELFORGE_CAM=7.6,7.0,-9.5,7.6,2.4,8.5,60 VOXELFORGE_DOF=14.5,1.8
echo "ALL_DONE"
