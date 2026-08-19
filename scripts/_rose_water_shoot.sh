#!/usr/bin/env bash
# ===========================================================================
# Rose — water evidence pair, ONE BINARY, ONE CAMERA, ONE VARIABLE (the map).
#
#   before = maps/river_sunset_dry.json  (the valley, no water anywhere)
#   after  = maps/river_sunset.json      (the same valley, river in it)
#
# Both frames come from the SAME exe with the SAME env; only VOXELFORGE_MAP_LOAD
# differs. That is the claim "water exists in the game now" in its honest form:
# nothing but the water blocks moved.
#
# Stale-binary gate inherited from _fl_v4_shoot_20260818.sh: refuse to shoot an
# exe older than the sources this pass edits.
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
OUT="$ROOT/_rose_water_20260818"
mkdir -p "$OUT"

EXE="target-rose/release/voxelforge.exe"
stamp() { date "+[%H:%M:%S]"; }

if [ ! -f "$EXE" ]; then
  echo "REFUSED  $EXE missing"; exit 2
fi

newest_src=0; newest_name=""
for f in sim/src/block.rs client/src/voxel.rs client/src/mapfile.rs maps/river_sunset.json; do
  t=$(stat -c %Y "$f" 2>/dev/null || echo 0)
  if [ "$t" -gt "$newest_src" ]; then newest_src=$t; newest_name=$f; fi
done
exe_t=$(stat -c %Y "$EXE")
echo "$(stamp) exe   mtime $(date -d @"$exe_t" '+%Y-%m-%d %H:%M:%S') size $(stat -c %s "$EXE")"
echo "$(stamp) src   newest $newest_name $(date -d @"$newest_src" '+%Y-%m-%d %H:%M:%S')"
if [ "$exe_t" -lt "$newest_src" ]; then
  echo "REFUSED  exe older than $newest_name"; exit 2
fi

# the binary must actually carry the water lane — a stale exe that predates
# BlockId::WATER loads river_sunset.json and SILENTLY SKIPS every 'water'
# voxel (mapfile returns None → apply_map counts it as skipped). The runtime
# gate below catches that from the loader's own printed block counts.
echo "$(stamp) stale-binary gate: PASS"

# ---- shared capture env (the v3/v4 look recipe) ---------------------------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0

# Camera: south bank, chest height on the ridge, looking north down the
# valley along the river's S. Framing knobs are overridable per-shot below.
CAM="${VOXELFORGE_ROSE_CAM:-34,15,60, 34,15,60, 30,4,18, 1}"
SUN="${VOXELFORGE_ROSE_SUN:-}"

shoot() { # $1=map  $2=tag
  local map="$1" tag="$2"
  local png="$OUT/${tag}.png"
  rm -f "$png"
  VOXELFORGE_CINE="$CAM" \
  VOXELFORGE_LOOK_SUN="$SUN" \
  VOXELFORGE_MAP_LOAD="$map" \
  VOXELFORGE_SHOT="$png" \
    "./$EXE" --play > "$OUT/${tag}.log" 2>&1
  if [ -f "$png" ]; then
    echo "$(stamp) shot $tag <- $map  $(stat -c %s "$png") bytes"
  else
    echo "$(stamp) FAILED $tag — see $OUT/${tag}.log"; tail -5 "$OUT/${tag}.log"
  fi
}

shoot maps/river_sunset_dry.json before
shoot maps/river_sunset.json   after
# noise floor: the SAME wet map shot a second time. Nothing differs but
# run-to-run nondeterminism (GPU timing, dither, particle phase), so
# |after - noise| is the floor any real before/after delta must clear.
shoot maps/river_sunset.json   noise

# ---- runtime water gate: the loader must have stamped the water voxels ----
# MAP_APPLY prints set/skipped; an exe that predates BlockId::WATER parses the
# file but skips every water voxel, which shows here as skipped=2011.
want_dry=$(python -c "import json;print(len(json.load(open('maps/river_sunset_dry.json'))['blocks']))")
want_wet=$(python -c "import json;print(len(json.load(open('maps/river_sunset.json'))['blocks']))")
ap() { grep -o 'MAP_APPLY set=[0-9]* skipped=[0-9]*' "$1" | head -1; }
echo "$(stamp) before: $(ap "$OUT/before.log")  (want set=$want_dry skipped=0)"
echo "$(stamp) after : $(ap "$OUT/after.log")  (want set=$want_wet skipped=0)"
ok=1
[ "$(grep -o 'MAP_APPLY set=[0-9]*' "$OUT/before.log" | grep -o '[0-9]*')" = "$want_dry" ] || ok=0
[ "$(grep -o 'MAP_APPLY set=[0-9]*' "$OUT/after.log"  | grep -o '[0-9]*')" = "$want_wet" ] || ok=0
[ "$(grep -o 'skipped=[0-9]*' "$OUT/after.log" | grep -o '[0-9]*')" = "0" ] || ok=0
if [ "$ok" != "1" ]; then
  echo "GATE FAIL — the exe did not stamp the water blocks (stale binary?)"
  exit 3
fi
echo "$(stamp) water-in-binary gate: PASS"

# ---- publish the CEO-facing pair at the paths the Director asked for ----
mkdir -p docs/assets/look
cp "$OUT/before.png" docs/assets/look/water_before.png
cp "$OUT/after.png"  docs/assets/look/water_after.png
ls -la docs/assets/look/water_before.png docs/assets/look/water_after.png

echo "$(stamp) pair done -> $OUT (+ docs/assets/look/water_{before,after}.png)"
