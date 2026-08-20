#!/usr/bin/env bash
# Foliage PRESENCE A/B: BEFORE (VOXELFORGE_FOLIAGE=off, no plants) vs AFTER (default, plants scattered).
# Same binary, same map, same static Cine camera, same shot time, and BOTH runs wind-frozen
# (VOXELFORGE_FOLIAGE_WIND_OFF=1) so the only difference between the frames is plants existing vs not.
#
# Camera matches _kevin_ingame_ab.sh: eye (20,1.7,13) -> aim (20,1.1,2), looking north along the
# flat-grass x=20 corridor, player/campfire behind the camera.
set -u
cd "$(dirname "$0")/.."   # project root

EXE=./target-kevin/release/voxelforge.exe
OUT=_kevin_foliage/presence
mkdir -p "$OUT"
CINE="20.0,1.7,13.0,20.0,1.7,13.0,20.0,1.1,2.0,1.0"

echo "=== BEFORE: foliage OFF (no plants) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE=off \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
VOXELFORGE_SHOT="$OUT/foliage_off.png" \
"$EXE" > "$OUT/foliage_off.log" 2>&1

echo "=== AFTER: foliage ON (plants, wind frozen) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
VOXELFORGE_SHOT="$OUT/foliage_on.png" \
"$EXE" > "$OUT/foliage_on.log" 2>&1

echo "=== run-log evidence ==="
grep -hE 'FOLIAGE plants|SHOT saved|CINE eye' "$OUT/foliage_off.log" "$OUT/foliage_on.log"

echo "=== metrics ==="
python - "$OUT/foliage_off.png" "$OUT/foliage_on.png" <<'PY'
import sys, hashlib
import numpy as np
from PIL import Image
a = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=np.int16)
b = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=np.int16)
assert a.shape == b.shape, f"shape mismatch {a.shape} vs {b.shape}"
d = np.abs(a - b).max(axis=2)
moved = (d > 8)
print(f"pixels_changed_pct={100.0*moved.mean():.4f}")
# localize: fraction of changed pixels that sit in the green (vegetation) hue band of the AFTER frame
g = (b[:,:,1] > b[:,:,0]) & (b[:,:,1] > b[:,:,2])          # green-dominant pixels in after
print(f"green_frac_after={100.0*g.mean():.4f}  changed_px_in_green={100.0*(moved&g).mean()/max(1e-9,moved.mean()):.2f}%")
def md5(p):
    return hashlib.md5(open(p,'rb').read()).hexdigest()
print(f"md5_before={md5(sys.argv[1])}")
print(f"md5_after={md5(sys.argv[2])}")
print(f"shape={a.shape[1]}x{a.shape[0]}")
PY

echo "=== contact sheet ==="
python scripts/contact_sheet.py --out "$OUT/contact_sheet.png" \
    --before "$OUT/foliage_off.png" --before-label "BEFORE  foliage off" \
    --before-file client/src/scene.rs --before-caption "VOXELFORGE_FOLIAGE=off (no plants)" \
    --after "$OUT/foliage_on.png" --after-label "AFTER  foliage on" \
    --after-file client/src/scene.rs --after-caption "default scatter (plants, wind frozen)"
echo "sheet -> $OUT/contact_sheet.png"
