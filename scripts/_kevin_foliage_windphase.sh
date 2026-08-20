#!/usr/bin/env bash
# Foliage lane close-out: PROVE plants exist AND they sway in-game, in pictures.
#
# Four frames, ONE binary, ONE static Cine camera, ONE map (maps/demo.json grass field):
#   1. foliage_off.png   VOXELFORGE_FOLIAGE=off          -> no plants (presence BEFORE)
#   2. foliage_on.png    default foliage, WIND_OFF=1     -> plants, frozen (presence AFTER)
#   3. wind_phase_a.png  wind ON, WIND_T0=0.0            -> sway phase A
#   4. wind_phase_b.png  wind ON, WIND_T0=1.8            -> sway phase B (~half sway period later)
#
# Proofs:
#   * presence = diff(1,2)          : plants appear (pixels change, localized to green band)
#   * sway     = diff(3,4)          : same plants, same camera, only wind PHASE differs -> any
#                                     movement above noise is real wind, not just "leaves exist".
#   * sway (control) = diff(2,3)    : frozen plants vs moving plants.
#
# Camera: eye (20,1.7,13) -> aim (20,1.1,2), looking NORTH (-z) along the flat-grass
# x=20 corridor. Looking -z is perpendicular to wind_dir (+x) so sway reads left-right.
set -u
cd "$(dirname "$0")/.." || exit 1   # project root

EXE=./target-kevin/release/voxelforge.exe
OUT=_kevin_foliage/ingame
mkdir -p "$OUT"
CINE="20.0,1.7,13.0,20.0,1.7,13.0,20.0,1.1,2.0,1.0"

if [ ! -f "$EXE" ]; then
  echo "FATAL: exe not built yet: $EXE" >&2
  exit 1
fi

echo "=== 1/4 foliage OFF (no plants) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE=off \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
VOXELFORGE_SHOT="$OUT/foliage_off.png" \
"$EXE" > "$OUT/foliage_off.log" 2>&1
echo "exit=$?  $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/foliage_off.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE')  $(grep -o 'SHOT saved.*' "$OUT/foliage_off.log" 2>/dev/null || echo 'NO-SHOT-LINE')"

echo "=== 2/4 foliage ON, wind frozen (plants present, static) ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_OFF=1 \
VOXELFORGE_SHOT="$OUT/foliage_on.png" \
"$EXE" > "$OUT/foliage_on.log" 2>&1
echo "exit=$?  $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/foliage_on.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE')  $(grep -o 'SHOT saved.*' "$OUT/foliage_on.log" 2>/dev/null || echo 'NO-SHOT-LINE')"

echo "=== 3/4 wind ON, phase T0=0.0 ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_T0=0.0 \
VOXELFORGE_SHOT="$OUT/wind_phase_a.png" \
"$EXE" > "$OUT/wind_phase_a.log" 2>&1
echo "exit=$?  $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/wind_phase_a.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE')  $(grep -o 'SHOT saved.*' "$OUT/wind_phase_a.log" 2>/dev/null || echo 'NO-SHOT-LINE')"

echo "=== 4/4 wind ON, phase T0=1.8 ==="
VOXELFORGE_PLAY=1 \
VOXELFORGE_MAP_LOAD="maps/demo.json" \
VOXELFORGE_NOHUD=1 \
VOXELFORGE_CINE="$CINE" \
VOXELFORGE_FOLIAGE_WIND_T0=1.8 \
VOXELFORGE_SHOT="$OUT/wind_phase_b.png" \
"$EXE" > "$OUT/wind_phase_b.log" 2>&1
echo "exit=$?  $(grep -o 'FOLIAGE plants=[0-9]*' "$OUT/wind_phase_b.log" 2>/dev/null || echo 'NO-FOLIAGE-LINE')  $(grep -o 'SHOT saved.*' "$OUT/wind_phase_b.log" 2>/dev/null || echo 'NO-SHOT-LINE')"

echo "=== metrics ==="
python - "$OUT/foliage_off.png" "$OUT/foliage_on.png" "$OUT/wind_phase_a.png" "$OUT/wind_phase_b.png" <<'PY'
import sys, hashlib
import numpy as np
from PIL import Image

def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.int16)
def md5(p):
    return hashlib.md5(open(p, 'rb').read()).hexdigest()
def moved_frac(a, b, thr=8):
    d = np.abs(a - b).max(axis=2)
    return (d > thr)

off, on, pha, phb = map(load, sys.argv[1:5])
assert off.shape == on.shape == pha.shape == phb.shape, "shape mismatch"

presence = moved_frac(off, on)
phase    = moved_frac(pha, phb)
control  = moved_frac(on, pha)          # frozen vs moving (sway_amp 0 -> 0.16)

def green_band_frac(frame, mask):
    g = (frame[:,:,1] > frame[:,:,0]) & (frame[:,:,1] > frame[:,:,2])
    tot = mask.sum()
    return (100.0 * (mask & g).sum() / max(1, tot)) if tot else 0.0

print(f"shape={on.shape[1]}x{on.shape[0]}")
print(f"presence_changed_pct={100.0*presence.mean():.4f}  (off vs on, localized-to-green={green_band_frac(on, presence):.2f}%)")
print(f"wind_phase_moved_pct={100.0*phase.mean():.4f}  (T0 0.0 vs 1.8, localized-to-green={green_band_frac(pha, phase):.2f}%)")
print(f"frozen_vs_moving_pct={100.0*control.mean():.4f}  (WIND_OFF vs wind on)")
for label, p in zip(["off","on","phase_a","phase_b"], sys.argv[1:5]):
    print(f"md5_{label}={md5(p)}")
PY

echo "=== outputs ==="
ls -la "$OUT"
