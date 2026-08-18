#!/usr/bin/env bash
# ===========================================================================
# Poppy — the VOXELFORGE_MAT_MAPS A/B: three scenes, ONE binary, two arms.
#
# WHAT IT SHOOTS
#   The three shipped look scenes (same camera + hour env as
#   scripts/_fl_v4_shoot_20260818.sh), each twice:
#       on  = VOXELFORGE_MAT_MAPS unset (authored _n/_r used where found)
#       off = VOXELFORGE_MAT_MAPS=off  (derived maps only)
#
#   Twice over, because the art drop's own manifest decides whether the lever
#   can move anything at all:
#     ARM shipped : VOXELFORGE_ATLAS_DIR unset -> assets/textures/blocks,
#                   whose manifest is tile_px 64 while voxel.rs pins TILE_PX 16.
#                   `tile_set()` returns None, `authored_map()` bails on its
#                   first line, and on/off MUST come out identical. This arm is
#                   the evidence for that, not an accident to be reported as a
#                   pass.
#     ARM atlas16 : VOXELFORGE_ATLAS_DIR=_poppy_matmaps/atlas16 (built by
#                   scripts/_poppy_matmaps_prep.py) -> 16px albedo the gate
#                   accepts + the 64px authored maps unchanged. Here the lever
#                   reaches the loader, so a 0.00 diff would be a real failure.
#
#   Plus a NULL PAIR per arm: `day` shot twice under IDENTICAL env. Its delta is
#   this session's capture noise floor; an on/off delta below it is the renderer
#   breathing, not the maps.
#
# BINARY
#   voxelforge.exe ONLY. voxelforge_shot.exe is the isolated hero-shot bin
#   (client/src/shot_main.rs) and does not declare `mod voxel` — MAT_MAPS is not
#   compiled into it, so it would return a guaranteed, meaningless 0.00.
#
# USAGE
#   scripts/_poppy_matmaps_shoot.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
PUB="$ROOT/docs/assets/look"
RAW="$ROOT/_poppy_matmaps/plates"
mkdir -p "$PUB" "$RAW"

stamp() { date "+[%H:%M:%S]"; }
mt()    { stat -c %Y "$1" 2>/dev/null || echo 0; }

EXE="${1:-target/release/voxelforge.exe}"
if [ ! -f "$EXE" ]; then
  echo "REFUSED  $EXE not found — this pass shoots the PLAY scene, which only the full binary renders."
  exit 2
fi

# ---- stale-binary gate ---------------------------------------------------
SRC="client/src/voxel.rs client/src/look.rs client/src/main.rs"
newest_src=0; newest_name=""
for f in $SRC; do t=$(mt "$f"); if [ "$t" -gt "$newest_src" ]; then newest_src=$t; newest_name=$f; fi; done
exe_t=$(mt "$EXE")
echo "$(stamp) exe   : $EXE"
echo "$(stamp)         mtime $(date -d @"$exe_t" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE") bytes"
echo "$(stamp) source: newest edited $newest_name  $(date -d @"$newest_src" '+%Y-%m-%d %H:%M:%S')"
if [ "$exe_t" -lt "$newest_src" ]; then
  echo "REFUSED  the exe is OLDER than $newest_name — it cannot contain the loader under test."
  exit 2
fi
echo "$(stamp) stale-binary gate: PASS"
echo

# ---- shared capture env (identical to the shipped v4 recipe) -------------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0

ARM=""          # set per arm below
OUTDIR=""

shoot() { # $1=scene $2=lever(on|off|onNULL) $3=prefix
  local scene="$1" lever="$2" prefix="$3"
  local png="$OUTDIR/${prefix}${scene}_${lever}.png"
  local log="$RAW/${ARM}_${scene}_${lever}.log"
  rm -f "$png"
  if [ "$lever" = "off" ]; then
    VOXELFORGE_MAT_MAPS=off VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  else
    VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  fi
  local art pbr
  art=$(grep -c '^BLOCK_ART file-backed' "$log" 2>/dev/null || echo 0)
  pbr=$(grep -c '^BLOCK_PBR authored' "$log" 2>/dev/null || echo 0)
  if [ -f "$png" ]; then
    echo "$(stamp) shot  $ARM $scene $lever -> $(basename "$png")  $(stat -c %s "$png") bytes   BLOCK_ART=$art BLOCK_PBR_authored=$pbr"
  else
    echo "$(stamp) FAILED $ARM $scene $lever — no frame written (see $(basename "$log"))"
  fi
}

run_arm() { # $1=arm name $2=outdir $3=prefix
  ARM="$1"; OUTDIR="$2"; local prefix="$3"
  echo "$(stamp) === ARM $ARM   ATLAS_DIR=${VOXELFORGE_ATLAS_DIR:-<default assets/textures/blocks>}"

  # scene 1: day (the shipped outdoor-noon camera + hour)
  export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
  export VOXELFORGE_LOOK_SUN=66,205,20000
  export VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
  export VOXELFORGE_LOOK_EXPOSURE=10.6
  shoot day on     "$prefix"
  shoot day off    "$prefix"
  shoot day onNULL "$prefix"     # null pair: identical env, identical lever
  unset VOXELFORGE_LOOK_SUN VOXELFORGE_LOOK_LIGHT VOXELFORGE_LOOK_EXPOSURE

  # scene 2: raking evening sun (the shipped GOLDEN hour)
  export VOXELFORGE_CINE="18,9,44, 18,9,44, 31,1.5,29, 1"
  shoot evening-raking on  "$prefix"
  shoot evening-raking off "$prefix"

  # scene 3: night, lit by fire
  export VOXELFORGE_CINE="36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1"
  export VOXELFORGE_LOOK_NIGHT=1
  shoot night-firelit on  "$prefix"
  shoot night-firelit off "$prefix"
  unset VOXELFORGE_LOOK_NIGHT
  echo
}

# ---- arm 1: the art drop exactly as it sits in the working tree ----------
unset VOXELFORGE_ATLAS_DIR
run_arm shipped "$RAW" "shipped_"

# ---- arm 2: the 16px-manifest set the TILE_PX gate accepts ---------------
export VOXELFORGE_ATLAS_DIR="_poppy_matmaps/atlas16"
run_arm atlas16 "$PUB" "matmaps_"
unset VOXELFORGE_ATLAS_DIR

echo "$(stamp) shot set complete"
echo "$(stamp)   published (atlas16): $PUB/matmaps_*_{on,off}.png"
echo "$(stamp)   raw (shipped arm)  : $RAW/shipped_*_{on,off}.png"
