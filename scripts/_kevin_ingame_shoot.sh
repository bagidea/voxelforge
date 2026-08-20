#!/usr/bin/env bash
# In-game foliage wind A/B — wind OFF vs wind ON, same frame, same camera.
# Run from the project root (assets + maps/demo.json resolve from CWD).
set -u
cd "$(dirname "$0")/.." || exit 1

EXE="target-kevin/release/voxelforge.exe"
OUT="_kevin_foliage/ingame"
mkdir -p "$OUT"

# 10 floats = fixed pose: eye_a(0-2), eye_b(3-5), aim(6-8), secs(9).
# eye at (14,1.6,3) looking down the flat-grass corridor x=14 toward +z.
CINE="14.0,1.6,3.0,14.0,1.6,3.0,14.0,1.1,27.0,1.0"

if [ ! -f "$EXE" ]; then
  echo "FATAL: exe not built yet: $EXE" >&2
  exit 1
fi

echo "=== BEFORE: wind OFF (VOXELFORGE_FOLIAGE_WIND_OFF=1) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD=maps/demo.json \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_SHOT="$OUT/before.png" \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
"$EXE" > "$OUT/before.log" 2>&1
echo "before exit=$?"

echo "=== AFTER: wind ON (default sway_amp) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD=maps/demo.json \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_SHOT="$OUT/after.png" \
VOXELFORGE_CINE="$CINE" \
"$EXE" > "$OUT/after.log" 2>&1
echo "after exit=$?"

echo "=== log check ==="
echo "before: $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/before.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE') / $(grep -o 'SHOT saved.*' "$OUT/before.log" 2>/dev/null || echo 'NO-SHOT-LINE')"
echo "after:  $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/after.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE') / $(grep -o 'SHOT saved.*' "$OUT/after.log" 2>/dev/null || echo 'NO-SHOT-LINE')"
echo "--- CINE line (before) ---"
grep 'CINE eye' "$OUT/before.log" 2>/dev/null || echo '(no CINE line)'
echo "--- error lines (before) ---"
grep -iE 'error|panic|not found' "$OUT/before.log" 2>/dev/null | head -5 || echo '(none)'

echo "=== outputs ==="
ls -la "$OUT"
