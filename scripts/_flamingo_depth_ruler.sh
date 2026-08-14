#!/usr/bin/env bash
# _flamingo_depth_ruler.sh — MEASURE the depth range the pinned framing actually
# looks through, so HAZE_START/HAZE_FULL are fitted to the set instead of guessed.
#
# METHOD, and it is the repo's own (look.rs:111 / `_flamingo_g7_probe.ps1`):
# `VOXELFORGE_LOOK_FOG=S,S+0.5` makes `FogFalloff::Linear` a HARD STEP at S —
# alpha is exactly 0.0 nearer than S and 1.0 past S+0.5. So a capture at S is the
# frame with everything beyond S painted haze colour, and the pixels that MOVED
# against a no-haze reference are exactly the pixels whose geometry is farther
# than S. Sweeping S therefore reads the frame's depth CDF straight off the
# screen, with no depth buffer readback and NO REBUILD — one binary, env only.
#
# WHAT IT CANNOT SEE, said out loud rather than hidden:
#   * A surface already the haze colour moves by ~0 and is undercounted. That
#     biases the far tail LOW (far geometry is closest to haze colour), so the
#     measured p95 is a FLOOR on the real depth, never an overestimate.
#   * Sky and the sky dome carry `fog_enabled = false`, so they never move at any
#     S and drop out of the population automatically — which is what we want:
#     the question is how deep the GEOMETRY is, not how far the sky is.
#
# The denominator is the S=1 capture (everything past 1.5 blocks hazed = all
# fog-receiving geometry in frame), not the pixel count — see `norm` in the
# python block.
#
# Usage:  BIN=./target/release/voxelforge.exe bash scripts/_flamingo_depth_ruler.sh
set -uo pipefail
cd "$(dirname "$0")/.."

BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-_flamingo_depthruler}"
CAM="${CAM:-35,-18,26}"          # the pinned vista framing every gate row is shot on
mkdir -p "$OUT"
[ -f "$BIN" ] || { echo "no binary $BIN"; exit 2; }
echo "BIN $BIN  $(ls -l --time-style=+%Y-%m-%d\ %H:%M:%S "$BIN" | awk '{print $5" bytes  "$6" "$7}')"
echo "CAM $CAM"

# S=9999 is the no-haze reference (nothing in a 320-block world is past it).
# S=1 is the all-geometry denominator. The rest is the ladder.
STEPS=(9999 1 4 8 12 16 20 24 28 32 40 48 56 64 72 80 96 112 128 160 200 256 320)

for S in "${STEPS[@]}"; do
  png="$OUT/d${S}.png"
  if [ -s "$png" ]; then echo "  skip S=$S (have it)"; continue; fi
  END=$(python -c "print(f'{$S+0.5:g}')")
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_CAM="$CAM" \
      VOXELFORGE_LOOK_QUALITY=ultra \
      VOXELFORGE_LOOK_FOG="$S,$END" \
      VOXELFORGE_SHOT="$png" \
      timeout 120 "$BIN" >"$OUT/d${S}.log" 2>&1
  rc=$?
  bytes=0; [ -s "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  S=$S exit=$rc bytes=$bytes"
  grep -aqE 'panicked|B0001' "$OUT/d${S}.log" && echo "     !! PANIC"
done

echo
echo "=== depth CDF ==="
OUT="$OUT" python scripts/_flamingo_depth_ruler.py
