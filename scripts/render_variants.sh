#!/usr/bin/env bash
# Sequential hero-frame render variants (env-driven, no recompile).
# Each run auto-screenshots at 3.2s and exits at 4.4s. We check exit code +
# verify the PNG mtime advanced so we never falsely report a good frame.
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
  if [[ -f "$out" ]]; then
    echo "OK   $out  ($(stat -c%s "$out") bytes)"
  else
    echo "MISS $out — no png (see logs_$out.txt)"
  fi
}

# v1: camera raised + pulled back, look down at island; ease exposure + fog.
run tune-v1.png \
  VOXELFORGE_CAM=7.5,6.0,-5.0,7.5,2.3,6.0,48 \
  VOXELFORGE_EXPOSURE=9.3 VOXELFORGE_FOG=0.04 \
  VOXELFORGE_DOF=9.3,2.8

# v2: a touch lower + wider, brighter shadows (lower ev100), gentler fog.
run tune-v2.png \
  VOXELFORGE_CAM=7.0,5.3,-4.5,7.5,2.4,6.0,52 \
  VOXELFORGE_EXPOSURE=9.0 VOXELFORGE_FOG=0.035 \
  VOXELFORGE_DOF=8.5,2.4

# v3: closer hero framing, sun a bit higher for softer key, less fog haze.
run tune-v3.png \
  VOXELFORGE_CAM=7.2,5.6,-4.0,7.3,2.6,5.5,50 \
  VOXELFORGE_SUN=26,200,12000 \
  VOXELFORGE_EXPOSURE=9.2 VOXELFORGE_FOG=0.03 \
  VOXELFORGE_DOF=8.0,2.8

echo "ALL_DONE"
