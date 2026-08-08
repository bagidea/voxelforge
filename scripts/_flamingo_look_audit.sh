#!/usr/bin/env bash
# Flamingo — honest look audit (2026-08-03).
# Shoots the REAL gameplay camera through the release binary that was just built
# from HEAD, with the look lane ON and OFF, so "what the player sees today" can be
# graded instead of assumed. Every frame comes out of `--play` (the same gate
# look.rs::enabled_for uses), not out of hero.rs.
set -uo pipefail
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
OUT=_flamingo_look_audit
mkdir -p "$OUT"
[[ -f "$EXE" ]] || { echo "NO EXE $EXE"; exit 2; }

shoot() { # name  extra-env...
  local name=$1; shift
  echo "--- $name"
  env "$@" VOXELFORGE_PLAY=1 VOXELFORGE_SHOT="$OUT/$name.png" \
    "$EXE" >"$OUT/$name.log" 2>&1
  local code=$?
  if [[ -f "$OUT/$name.png" ]]; then
    echo "OK   $OUT/$name.png ($(stat -c%s "$OUT/$name.png")b) exit=$code"
  else
    echo "MISS $OUT/$name.png exit=$code"; tail -5 "$OUT/$name.log"
  fi
  grep -l -E 'panicked|B0001' "$OUT/$name.log" >/dev/null 2>&1 && echo "  !! PANIC in log"
}

# 1. what the player actually sees at spawn, look lane ON (shipped defaults)
shoot look-on-boot

# 2. same frame, look lane OFF — the honest before/after of the whole look lane
shoot look-off-boot VOXELFORGE_LOOK_DISABLE=1

# 3. a vista: boom pulled back + angled down, to read fog / sun / shadow length
shoot look-on-vista VOXELFORGE_LOOK_CAM=35,-18,26

# 4. same vista, look off
shoot look-off-vista VOXELFORGE_LOOK_DISABLE=1 VOXELFORGE_LOOK_CAM=35,-18,26

# 5. ultra tier explicitly (TAA + SSAO + volumetric on)
shoot look-ultra-vista VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_LOOK_CAM=35,-18,26

echo "=== done ==="
ls -la "$OUT"/*.png 2>/dev/null
