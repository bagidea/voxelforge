#!/usr/bin/env bash
# _rose_sky_shoot.sh — A1/A2 before-after proof from ONE binary.
#
# Three deterministic play-path frames (VOXELFORGE_PLAY=1, screenshot_once at
# t=3.2s) out of the SAME dev exe, varying only the two lane env hooks:
#
#   flat     VOXELFORGE_LOOK_SKYGRAD=off  VOXELFORGE_LOOK_VFOG=off   (baseline: flat ClearColor, no fog medium)
#   dome     <SKYGRAD on, default>        VOXELFORGE_LOOK_VFOG=off   (A1: gradient dome; fog off to isolate the sky)
#   domefog  <both default on>                                        (A2: dome + play FogVolume -> god-ray medium)
#
# A1 is read off flat-vs-dome (gradient swing + horizon seam); A2 off
# dome-vs-domefog (the volumetric term lighting up). This mirrors gate3_shoot's
# boot shot (VOXELFORGE_PLAY=1) but toggles the lane hooks gate3 leaves fixed.
#
# Usage:
#   BIN=./target-rose/debug/voxelforge.exe bash scripts/_rose_sky_shoot.sh
set -uo pipefail

# Default matches gate3_shoot.sh convention (./target/release/voxelforge.exe);
# override BIN for a shot binary or a per-lane target dir.
BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-docs/assets/gate3}"
LOGS="${LOGS:-_rose_sky_logs}"
QUALITY="${QUALITY:-high}"

mkdir -p "$OUT" "$LOGS"

if [ ! -f "$BIN" ]; then
  echo "✗ binary not found: $BIN" >&2
  exit 1
fi

shoot() {
  local png="$1" label="$2"; shift 2
  local log="$LOGS/${label}.log"
  echo "=== SHOOT $label  ->  $png ==="
  echo "  bin=$BIN quality=$QUALITY"
  env "$@" \
      VOXELFORGE_PLAY=1 \
      VOXELFORGE_SHOT="$png" \
      VOXELFORGE_LOOK_QUALITY="$QUALITY" \
      "$BIN" >"$log" 2>&1 || true
  local rc=$?
  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  exit=$rc bytes=$bytes"
  grep -aE 'LOOK sky-dome spawned|LOOK play FogVolume spawned|SHOT saved' "$log" \
    || echo "  (no LOOK/SHOT markers — check $log)"
  if grep -aqE 'panicked|B0001' "$log"; then
    echo "  ✗ PANIC in $log"
  fi
}

# flat  — the before: single-colour ClearColor sky, no volumetric medium
shoot "$OUT/sky-flat.png"    flat    VOXELFORGE_LOOK_SKYGRAD=off VOXELFORGE_LOOK_VFOG=off
# dome  — A1: 3-stop gradient dome; fog off so the sky delta is the dome alone
shoot "$OUT/sky-dome.png"    dome    VOXELFORGE_LOOK_VFOG=off
# domefog — A2: dome + play FogVolume so the installed VolumetricFog ray-marches
shoot "$OUT/sky-domefog.png" domefog

echo ""
echo "=== frames in $OUT ==="
ls -la "$OUT"/sky-*.png 2>/dev/null
