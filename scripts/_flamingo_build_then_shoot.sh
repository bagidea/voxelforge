#!/usr/bin/env bash
# Flamingo — build HEAD+streaming fix, then shoot the look audit in ONE detached
# chain so a dying session can't orphan the run (lesson from the 2026-08-03 audit:
# the build was killed mid-link and "still linking" got reported instead).
# Verdict is decided by `grep '^error'` on the build log — never by tail/exit-through-pipe.
set -uo pipefail
cd "$(dirname "$0")/.."
OUT=_flamingo_look_audit
mkdir -p "$OUT"
BLOG="$OUT/rebuild.log"
EXE=target/release/voxelforge.exe

echo "=== BUILD START $(date -Is) ===" | tee "$BLOG"
cargo build --release -p voxelforge >>"$BLOG" 2>&1
BUILD_RC=$?
echo "=== BUILD END rc=$BUILD_RC $(date -Is) ===" >>"$BLOG"

if grep -q '^error' "$BLOG"; then
  echo "BUILD FAILED — compiler errors:" | tee -a "$OUT/verdict.txt"
  grep '^error' -A4 "$BLOG" | tee -a "$OUT/verdict.txt"
  exit 1
fi
if [[ $BUILD_RC -ne 0 ]]; then
  echo "BUILD FAILED rc=$BUILD_RC (no '^error' line — link/toolchain failure)" | tee -a "$OUT/verdict.txt"
  exit 1
fi
echo "BUILD OK exe=$(stat -c%s "$EXE")b mtime=$(stat -c%y "$EXE")" | tee "$OUT/verdict.txt"

shoot() { # name  extra-env...
  local name=$1; shift
  rm -f "$OUT/$name.png"
  env "$@" VOXELFORGE_PLAY=1 VOXELFORGE_SHOT="$OUT/$name.png" \
    "$EXE" >"$OUT/$name.log" 2>&1
  local code=$?
  if [[ -f "$OUT/$name.png" ]]; then
    echo "OK   $name.png ($(stat -c%s "$OUT/$name.png")b) exit=$code" | tee -a "$OUT/verdict.txt"
  else
    echo "MISS $name.png exit=$code" | tee -a "$OUT/verdict.txt"
    grep -m2 -E 'panicked|error\[B' "$OUT/$name.log" | tee -a "$OUT/verdict.txt"
  fi
}

shoot look-on-boot
shoot look-off-boot   VOXELFORGE_LOOK_DISABLE=1
shoot look-on-vista   VOXELFORGE_LOOK_CAM=35,-18,26
shoot look-off-vista  VOXELFORGE_LOOK_DISABLE=1 VOXELFORGE_LOOK_CAM=35,-18,26
shoot look-ultra-vista VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_LOOK_CAM=35,-18,26

echo "=== CHAIN DONE $(date -Is) ===" | tee -a "$OUT/verdict.txt"
