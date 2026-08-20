#!/usr/bin/env bash
# In-game wind A/B: BEFORE (wind off, sway_amp=0) vs AFTER (wind on, sway_amp=0.16).
# Same binary (target-kevin/release/voxelforge), same map (demo.json grass field),
# same static Cine camera, same shot time. Only VOXELFORGE_FOLIAGE_WIND_OFF differs,
# so any pixel movement between the two frames is pure wind.
#
# Camera: eye (20, 1.7, 13) -> aim (20, 1.1, 2), looking NORTH along the flat-grass
# x=20 corridor. North keeps the center-spawned player/campfire behind the camera,
# and looking -z is perpendicular to wind_dir (+x) so sway reads left-right.
set -u
cd "$(dirname "$0")/.."   # project root

EXE=./target-kevin/release/voxelforge.exe
OUT=_kevin_foliage/ingame
mkdir -p "$OUT"
CINE="20.0,1.7,13.0,20.0,1.7,13.0,20.0,1.1,2.0,1.0"

echo "=== BEFORE: wind OFF ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
VOXELFORGE_SHOT="$OUT/wind_off.png" \
"$EXE" > "$OUT/wind_off.log" 2>&1

echo "=== AFTER: wind ON ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_SHOT="$OUT/wind_on.png" \
"$EXE" > "$OUT/wind_on.log" 2>&1

echo "=== run-log evidence ==="
grep -hE 'FOLIAGE plants|SHOT saved|CINE eye' "$OUT/wind_off.log" "$OUT/wind_on.log"

echo "=== metrics ==="
python - "$OUT/wind_off.png" "$OUT/wind_on.png" <<'PY'
import sys, hashlib
import numpy as np
from PIL import Image
a = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=np.int16)
b = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=np.int16)
assert a.shape == b.shape, f"shape mismatch {a.shape} vs {b.shape}"
moved = (np.abs(a - b).max(axis=2) > 8).mean() * 100.0
def md5(p):
    return hashlib.md5(open(p, 'rb').read()).hexdigest()
print(f"pixels_moved_pct={moved:.4f}")
print(f"md5_before={md5(sys.argv[1])}")
print(f"md5_after={md5(sys.argv[2])}")
print(f"shape={a.shape[1]}x{a.shape[0]}")
PY

echo "=== contact sheet ==="
python scripts/contact_sheet.py --out "$OUT/contact_sheet.png" \
    --before "$OUT/wind_off.png" --before-label "BEFORE  wind off" \
    --before-file client/src/scene.rs --before-caption "VOXELFORGE_FOLIAGE_WIND_OFF=1 (sway_amp=0)" \
    --after "$OUT/wind_on.png" --after-label "AFTER  wind on" \
    --after-file client/src/scene.rs --after-caption "default (sway_amp=0.16)"
echo "sheet -> $OUT/contact_sheet.png"
