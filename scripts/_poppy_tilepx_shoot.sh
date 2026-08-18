#!/usr/bin/env bash
# ===========================================================================
# Poppy — manifest-driven tile size: what did 64 px buy, and what did the maps?
#
# THE THREE ARMS. One binary, one camera and one hour per scene; the ONLY thing
# that moves between arms is which art set the manifest points at and whether
# the authored maps are allowed to bind.
#
#   head16    VOXELFORGE_ATLAS_DIR=assets/textures/blocks_16px_backup
#             HEAD's art verbatim: 16 px albedo, and NO `_n`/`_r` files in the
#             folder at all, so every map is derived. This is what shipped.
#
#   art64off  VOXELFORGE_ATLAS_DIR unset (assets/textures/blocks, tile_px 64)
#             VOXELFORGE_MAT_MAPS=off
#             Monanisa's 64 px albedo, maps still derived. The difference from
#             `head16` is the ALBEDO UPGRADE ALONE.
#
#   art64on   VOXELFORGE_ATLAS_DIR unset, MAT_MAPS unset
#             The full drop. The difference from `art64off` is THE AUTHORED
#             NORMAL/ROUGHNESS MAPS ALONE.
#
# Splitting it this way is the whole point: a single before/after would credit
# the maps for what the albedo did, and vice versa.
#
# Every arm also records `BLOCK_ART file-backed` and `BLOCK_PBR authored` counts
# straight out of its own log, because "the art set loaded" is a claim the run
# can make for itself and I should not be making on its behalf.
#
# NULL PAIR: `day` shot twice in the art64on arm under identical env. The scene
# animates (fire, foliage), so two identical runs already disagree — that delta
# is this session's floor and nothing below it counts.
#
# BINARY: voxelforge.exe only. voxelforge_shot.exe declares no `mod voxel`.
#
# USAGE
#   scripts/_poppy_tilepx_shoot.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."

EXE="${1:-target-poppy/release/voxelforge.exe}"
OUT="_poppy_tilepx"
RAW="$OUT/plates"
mkdir -p "$RAW"

stamp() { date "+[%H:%M:%S]"; }
mt()    { stat -c %Y "$1" 2>/dev/null || echo 0; }

[ -f "$EXE" ] || { echo "REFUSED  $EXE not found"; exit 2; }

# ---- stale-binary gate ---------------------------------------------------
newest_src=0; newest_name=""
for f in client/src/voxel.rs client/src/block_atlas.rs client/src/look.rs client/src/main.rs; do
  t=$(mt "$f"); if [ "$t" -gt "$newest_src" ]; then newest_src=$t; newest_name=$f; fi
done
exe_t=$(mt "$EXE")
echo "$(stamp) exe   : $EXE"
echo "$(stamp)         mtime $(date -d @"$exe_t" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE") bytes"
echo "$(stamp)         sha256 $(sha256sum "$EXE" | cut -c1-16)"
echo "$(stamp) source: newest edited $newest_name  $(date -d @"$newest_src" '+%Y-%m-%d %H:%M:%S')"
if [ "$exe_t" -lt "$newest_src" ]; then
  echo "REFUSED  the exe is OLDER than $newest_name — it cannot contain the change under test."
  exit 2
fi
echo "$(stamp) stale-binary gate: PASS"
echo

# ---- shared capture env (identical to the shipped v4 / matmaps recipe) ----
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0

ARM=""

shoot() { # $1=scene  $2=suffix ("" or NULL)
  local scene="$1" suffix="${2:-}"
  local name="${ARM}_${scene}${suffix}"
  local png="$RAW/${name}.png" log="$RAW/${name}.log"
  rm -f "$png"
  "./$EXE" --play > "$log" 2>&1
  local art pbr px
  art=$(grep -c '^BLOCK_ART file-backed' "$log" 2>/dev/null || echo 0)
  pbr=$(grep -c '^BLOCK_PBR authored' "$log" 2>/dev/null || echo 0)
  px=$(grep -m1 '^BLOCK_ART file-backed' "$log" | sed -n 's/.*tile_px=\([0-9]*\).*/\1/p')
  if [ -f "$png" ]; then
    echo "$(stamp) shot  $name  $(stat -c %s "$png") bytes   file-backed=$art tile_px=${px:-none} BLOCK_PBR_authored=$pbr"
  else
    echo "$(stamp) FAILED $name — no frame written; tail:"
    tail -8 "$log" | sed 's/^/          /'
  fi
}
# VOXELFORGE_SHOT has to be set per shot; keep it next to the run.
shoot_to() { export VOXELFORGE_SHOT="$RAW/${ARM}_${1}${2:-}.png"; shoot "$@"; unset VOXELFORGE_SHOT; }

run_arm() { # $1=arm  $2=1 to also take the null pair
  ARM="$1"
  echo "$(stamp) === ARM $ARM   ATLAS_DIR=${VOXELFORGE_ATLAS_DIR:-<default assets/textures/blocks>}  MAT_MAPS=${VOXELFORGE_MAT_MAPS:-<unset>}"

  # scene 1: day (the shipped outdoor-noon camera + hour)
  export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
  export VOXELFORGE_LOOK_SUN=66,205,20000
  export VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
  export VOXELFORGE_LOOK_EXPOSURE=10.6
  shoot_to day
  [ "${2:-0}" = "1" ] && shoot_to day NULL
  unset VOXELFORGE_LOOK_SUN VOXELFORGE_LOOK_LIGHT VOXELFORGE_LOOK_EXPOSURE

  # scene 2: raking evening sun (the shipped GOLDEN hour)
  export VOXELFORGE_CINE="18,9,44, 18,9,44, 31,1.5,29, 1"
  shoot_to evening-raking

  # scene 3: night, lit by fire
  export VOXELFORGE_CINE="36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1"
  export VOXELFORGE_LOOK_NIGHT=1
  shoot_to night-firelit
  unset VOXELFORGE_LOOK_NIGHT
  echo
}

# ---- arm 1: HEAD's 16 px set, derived maps -------------------------------
export VOXELFORGE_ATLAS_DIR="assets/textures/blocks_16px_backup"
unset VOXELFORGE_MAT_MAPS
run_arm head16

# ---- arm 2: the 64 px albedo, derived maps -------------------------------
unset VOXELFORGE_ATLAS_DIR
export VOXELFORGE_MAT_MAPS=off
run_arm art64off

# ---- arm 3: the 64 px albedo + the authored maps -------------------------
unset VOXELFORGE_MAT_MAPS
run_arm art64on 1

echo "$(stamp) shot set complete -> $RAW"
