#!/usr/bin/env bash
# ===========================================================================
# Poppy — the MAT_MAPS A/B re-shot on the REAL art, at last.
#
# WHY THIS FILE EXISTS AND `_poppy_matmaps_shoot.sh` IS NOT ENOUGH.
#   That pass had to route around the engine: `voxel.rs` pinned TILE_PX to 16,
#   so `assets/textures/blocks` (a 64 px manifest) was thrown away whole and the
#   only arm where the lever could move anything was `_poppy_matmaps/atlas16`,
#   a DOWNSAMPLED proxy built by `_poppy_matmaps_prep.py`. The verdict that came
#   out of it — 0 of 6 pairs pass — is therefore a verdict about a 16 px proxy,
#   not about the art Monanisa shipped.
#
#   55e08e3 separated the two constants (`PROC_TILE_PX` = the bake size, stays
#   16; `source_tile_px()` = whatever the manifest says). The 64 px set can now
#   reach a frame. This shoots the SAME three scenes, the SAME lever, with
#   VOXELFORGE_ATLAS_DIR unset — i.e. on `assets/textures/blocks` itself.
#
# THE BINARY GATE IS A STRING SCAN, NOT AN mtime.
#   mtime says when a file was written, not what is in it, and this lane has
#   already been burned once by a plate shot from a binary that predated the fix
#   under test. So the gate greps the exe for a marker that only exists AFTER
#   55e08e3 (`LOD atlas capped`, from `atlas_tile_px()`) and for the line the
#   commit DELETED (`file set ignored, procedural tiles kept`). Both must agree.
#
# EVERY SHOT IS SELF-CHECKED.
#   A frame that renders with the art silently dropped looks like a result and
#   is not one, so each log is asserted for
#     `BLOCK_ART file-backed ... tile_px=64`   (the manifest actually loaded)
#     `BLOCK_PBR authored ...` count > 0       (the _n/_r maps actually loaded)
#   and any shot that misses either is printed as BAD, not counted.
#
# NULL PAIR PER SCENE, ALWAYS.
#   `on` twice under identical env. Its delta is this session's capture noise;
#   the judge refuses without it, and rightly. Named `<stem>_onNULL{A,B}` so
#   `_fl_matmaps_judge.py`'s borrowed-floor check sees the same stem.
#
# USAGE
#   scripts/_poppy_matmaps64_shoot.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
PUB="$ROOT/docs/assets/look"
RAW="$ROOT/_poppy_matmaps/plates64"
mkdir -p "$PUB" "$RAW"

stamp() { date "+[%H:%M:%S]"; }

EXE="${1:-target-poppy/release/voxelforge.exe}"
PREFIX="matmaps64_"

[ -f "$EXE" ] || { echo "REFUSED  $EXE not found"; exit 2; }

# ---- binary-provenance gate: does this exe CONTAIN the fix? --------------
NEW_MARK="LOD atlas capped"                             # added by 55e08e3
OLD_MARK="file set ignored, procedural tiles kept"      # deleted by 55e08e3
has_new=$(grep -c -a -F "$NEW_MARK" "$EXE" || true)
has_old=$(grep -c -a -F "$OLD_MARK" "$EXE" || true)
echo "$(stamp) exe   : $EXE"
echo "$(stamp)         mtime $(date -d @"$(stat -c %Y "$EXE")" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE") bytes"
echo "$(stamp)         sha256 $(sha256sum "$EXE" | cut -c1-32)"
echo "$(stamp)         marker '$NEW_MARK' = $has_new   (want >=1)"
echo "$(stamp)         marker '$OLD_MARK' = $has_old   (want 0)"
if [ "$has_new" -lt 1 ] || [ "$has_old" -ne 0 ]; then
  echo "REFUSED  this binary predates 55e08e3 — it cannot render the 64px set."
  exit 2
fi
echo "$(stamp) binary-provenance gate: PASS"
echo

# ---- shared capture env (identical to _poppy_matmaps_shoot.sh) ----------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0
unset VOXELFORGE_ATLAS_DIR      # ← the whole point: the shipped 64px art

BAD=0

shoot() { # $1=scene  $2=tag  $3=lever(on|off)
  local scene="$1" tag="$2" lever="$3"
  local png="$PUB/${PREFIX}${scene}_${tag}.png"
  local log="$RAW/${scene}_${tag}.log"
  rm -f "$png"
  if [ "$lever" = "off" ]; then
    VOXELFORGE_MAT_MAPS=off VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  else
    VOXELFORGE_SHOT="$png" "./$EXE" --play > "$log" 2>&1
  fi
  local art64 pbr note
  art64=$(grep -c '^BLOCK_ART file-backed.*tile_px=64' "$log" 2>/dev/null | head -1)
  pbr=$(grep -c '^BLOCK_PBR authored' "$log" 2>/dev/null | head -1)
  note=""
  [ "$art64" -lt 1 ] && note="$note  !!NO-64PX-ART"
  # `off` is the derived-map arm: BLOCK_PBR authored is EXPECTED to be 0 there.
  if [ "$lever" = "on" ] && [ "$pbr" -lt 1 ]; then note="$note  !!NO-AUTHORED-PBR"; fi
  if [ ! -f "$png" ]; then note="$note  !!NO-FRAME"; fi
  [ -n "$note" ] && BAD=$((BAD + 1))
  printf '%s shot  %-14s %-8s -> %-38s %8s B   tile_px64=%s authored_pbr=%s%s\n' \
    "$(stamp)" "$scene" "$tag" "$(basename "$png")" \
    "$(stat -c %s "$png" 2>/dev/null || echo 0)" "$art64" "$pbr" "$note"
}

scene_env() {
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

echo "$(stamp) === ARM shipped-64px   ATLAS_DIR=<default assets/textures/blocks>"
for scene in day evening-raking night-firelit; do
  scene_env "$scene"
  shoot "$scene" off     off
  shoot "$scene" on      on
  shoot "$scene" onNULLA on
  shoot "$scene" onNULLB on
  scene_unenv
done
echo
echo "$(stamp) shot set complete — $BAD bad shot(s)"
echo "$(stamp)   published: $PUB/${PREFIX}<scene>_{off,on,onNULLA,onNULLB}.png"
echo "$(stamp)   logs     : $RAW/<scene>_<tag>.log"
[ "$BAD" -eq 0 ] || exit 3
