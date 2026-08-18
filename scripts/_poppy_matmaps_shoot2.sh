#!/usr/bin/env bash
# ===========================================================================
# Poppy — the two things the matmaps verdict was missing.
#
# PASS A — A NULL PAIR PER SCENE.
#   `_poppy_matmaps_shoot.sh` only took one, on `day`. So evening-raking and
#   night-firelit had no floor of their own and the judge refused them (exit 2),
#   and night's M1 "PASS" leaned on day's floor — a borrowed floor is not a
#   result. This shoots BOTH members fresh, back to back, under the plate's own
#   env: `<scene>_onNULLA` then `<scene>_onNULLB`, same lever, same camera, same
#   hour. Their delta is that scene's capture noise, measured, not inherited.
#
# PASS B — WHICH MAP ATE THE VALUE SPAN.
#   atlas16/night-firelit failed M4 at -2.570 against a 1.159 floor with `_n`
#   and `_r` both live, so the number convicts a pair of files, not a file.
#   Here each one runs alone against its own `off`, out of the split dirs
#   scripts/_poppy_matmaps_split.py builds (identical albedo, identical
#   manifest — only which map the loader can find differs).
#
# THE BINARY IS PINNED, ON PURPOSE.
#   A noise floor is a property of ONE binary. These plates floor the ones shot
#   at 08:03 by target/release/voxelforge.exe, so this run re-uses that exact
#   file and checks its sha256 instead of the usual "exe newer than source"
#   gate — client/src/voxel.rs has been edited SINCE (that is the TILE_PX work,
#   which builds into target-poppy/), and a newer binary would make the floor
#   measure the rebuild rather than the renderer breathing.
#
# PASS C — a null pair for each split arm too.
#   The judge keys "is this floor borrowed?" off the plate's stem, and it is
#   right to: atlas16_nonly is a different atlas dir, so atlas16's floor is not
#   automatically its floor. One null pair per split arm, and the BORROWED
#   FLOOR banner comes off pass B's verdicts.
#
# USAGE
#   scripts/_poppy_matmaps_shoot2.sh [exe] [pass]     pass = all (default)|A|B|C
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
PUB="$ROOT/docs/assets/look"
RAW="$ROOT/_poppy_matmaps/plates"
mkdir -p "$PUB" "$RAW"

stamp() { date "+[%H:%M:%S]"; }

EXE="${1:-target/release/voxelforge.exe}"
PASS="${2:-all}"
WANT_SHA="c46a701597b869fafa617e41d40c0a32"   # the binary that shot matmaps_*_{on,off}.png
want() { [ "$PASS" = "all" ] || [ "$PASS" = "$1" ]; }

[ -f "$EXE" ] || { echo "REFUSED  $EXE not found"; exit 2; }
GOT_SHA=$(sha256sum "$EXE" | cut -c1-32)
echo "$(stamp) exe   : $EXE"
echo "$(stamp)         mtime $(date -d @"$(stat -c %Y "$EXE")" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE") bytes"
echo "$(stamp)         sha256[0:32] $GOT_SHA  (want $WANT_SHA)"
if [ "$GOT_SHA" != "$WANT_SHA" ]; then
  echo "REFUSED  this is not the binary that shot the plates — a floor from another build floors nothing."
  exit 2
fi
echo "$(stamp) binary-provenance gate: PASS"
echo

# ---- shared capture env (identical to _poppy_matmaps_shoot.sh) -----------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0

ARM=""; OUTDIR=""; PREFIX=""

shoot() { # $1=scene  $2=tag (onNULLA|onNULLB|nonly_on|...)  $3=off|on
  local scene="$1" tag="$2" lever="$3"
  local png="$OUTDIR/${PREFIX}${scene}_${tag}.png"
  local log="$RAW/${ARM}_${scene}_${tag}.log"
  rm -f "$png"
  if [ "$lever" = "off" ]; then
    VOXELFORGE_MAT_MAPS=off VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  else
    VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  fi
  local art pbr
  art=$(grep -c '^BLOCK_ART file-backed' "$log" 2>/dev/null | head -1)
  pbr=$(grep -c '^BLOCK_PBR authored' "$log" 2>/dev/null | head -1)
  if [ -f "$png" ]; then
    echo "$(stamp) shot  $ARM $scene $tag -> $(basename "$png")  $(stat -c %s "$png") bytes   BLOCK_ART=$art BLOCK_PBR_authored=$pbr"
  else
    echo "$(stamp) FAILED $ARM $scene $tag — no frame written (see $(basename "$log"))"
  fi
}

# Each scene's env, factored out so the null pair cannot drift from the plate.
scene_env() { # $1=scene  ("set" is the only thing this does; caller unsets)
  case "$1" in
    day)
      export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
      export VOXELFORGE_LOOK_SUN=66,205,20000
      export VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
      export VOXELFORGE_LOOK_EXPOSURE=10.6
      ;;
    evening-raking)
      export VOXELFORGE_CINE="18,9,44, 18,9,44, 31,1.5,29, 1"
      ;;
    night-firelit)
      export VOXELFORGE_CINE="36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1"
      export VOXELFORGE_LOOK_NIGHT=1
      ;;
  esac
}
scene_unenv() {
  unset VOXELFORGE_LOOK_SUN VOXELFORGE_LOOK_LIGHT VOXELFORGE_LOOK_EXPOSURE VOXELFORGE_LOOK_NIGHT
}

# =========================== PASS A — null pairs ===========================
null_arm() { # $1=arm  $2=outdir  $3=prefix  ($4 = ATLAS_DIR or empty)
  ARM="$1"; OUTDIR="$2"; PREFIX="$3"
  if [ -n "${4:-}" ]; then export VOXELFORGE_ATLAS_DIR="$4"; else unset VOXELFORGE_ATLAS_DIR; fi
  echo "$(stamp) === PASS A / ARM $ARM   ATLAS_DIR=${VOXELFORGE_ATLAS_DIR:-<default assets/textures/blocks>}"
  for scene in day evening-raking night-firelit; do
    scene_env "$scene"
    shoot "$scene" onNULLA on
    shoot "$scene" onNULLB on
    scene_unenv
  done
  echo
}

if want A; then
  null_arm shipped "$RAW" "shipped_" ""
  null_arm atlas16 "$PUB" "matmaps_" "_poppy_matmaps/atlas16"
fi

# ====================== PASS B — one channel at a time =====================
ARM="atlas16"; OUTDIR="$PUB"; PREFIX="matmaps_"
for variant in nonly ronly; do
  want B || break
  export VOXELFORGE_ATLAS_DIR="_poppy_matmaps/atlas16_$variant"
  echo "$(stamp) === PASS B / $variant   ATLAS_DIR=$VOXELFORGE_ATLAS_DIR"
  scene_env night-firelit
  shoot night-firelit "${variant}_on"  on
  shoot night-firelit "${variant}_off" off
  scene_unenv
done
unset VOXELFORGE_ATLAS_DIR
echo

# ================= PASS C — each split arm's own null pair =================
for variant in nonly ronly; do
  want C || break
  export VOXELFORGE_ATLAS_DIR="_poppy_matmaps/atlas16_$variant"
  echo "$(stamp) === PASS C / $variant null   ATLAS_DIR=$VOXELFORGE_ATLAS_DIR"
  scene_env night-firelit
  shoot night-firelit "${variant}_onNULLA" on
  shoot night-firelit "${variant}_onNULLB" on
  scene_unenv
done
unset VOXELFORGE_ATLAS_DIR
echo

echo "$(stamp) shot set complete"
echo "$(stamp)   published : $PUB/matmaps_*_onNULL{A,B}.png  +  matmaps_night-firelit_{nonly,ronly}_{on,off}.png"
echo "$(stamp)   raw       : $RAW/shipped_*_onNULL{A,B}.png"
