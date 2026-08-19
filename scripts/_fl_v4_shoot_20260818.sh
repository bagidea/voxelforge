#!/usr/bin/env bash
# ===========================================================================
# Flamingo — v4 look pass: the three-plate before/after, ONE BINARY.
#
# WHAT IT SHOOTS
#   The same three scenes, same camera, same hour env as the shipped v3 set
#   (scripts/_poppy_lookv3_shoot.cmd) — outdoor-noon / evening-raking /
#   night-firelit — twice each:
#       before = VOXELFORGE_LOOK_GEN=v3   (the shipped generation)
#       after  = VOXELFORGE_LOOK_GEN=v4   (this pass; also the new default)
#   ONE exe renders both. A before/after that needs two builds is showing you
#   two builds, not one change — that is the whole reason look.rs carries the
#   LookGen switch, and the reason EV_TRIM_V4 is applied after the env exposure
#   read rather than folded into the hour table (see its doc comment).
#
#   Plus ONE NULL PAIR: outdoor-noon shot twice under IDENTICAL env/gen. Its
#   pixel delta is this session's capture noise floor. Any v3→v4 delta smaller
#   than the null delta is not a change, it is the renderer breathing — and
#   without the null there is no way to tell those apart.
#
# STALE-BINARY GATE (the scar this exists for)
#   `_poppy_shotset_before` was a binary hours older than the source it was
#   supposed to represent, and its "before/after" was two different games, not
#   one change. So this script REFUSES to shoot with an exe older than the
#   sources the pass edits, and prints the exe's mtime + size on every plate.
#
# USAGE
#   scripts/_fl_v4_shoot_20260818.sh [exe]        # exe defaults to newest valid
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
OUT="$ROOT/_fl_v4_20260818/plates"
mkdir -p "$OUT"

SRC="client/src/look.rs client/src/hero.rs client/src/voxel.rs"

stamp() { date "+[%H:%M:%S]"; }
mt()    { stat -c %Y "$1" 2>/dev/null || echo 0; }

# ---- pick the exe --------------------------------------------------------
EXE="${1:-}"
if [ -z "$EXE" ]; then
  best=""; bestt=0
  for c in target-poppy/release/voxelforge.exe target/release/voxelforge.exe \
           target-pixel/release/voxelforge.exe; do
    [ -f "$c" ] || continue
    t=$(mt "$c"); if [ "$t" -gt "$bestt" ]; then bestt=$t; best=$c; fi
  done
  EXE="$best"
fi
if [ -z "$EXE" ] || [ ! -f "$EXE" ]; then
  echo "REFUSED  no voxelforge.exe found (looked in target-poppy/ target/ target-pixel/)"
  echo "         this pass shoots the PLAY scene, which only the full binary renders;"
  echo "         voxelforge_shot.exe renders hero.rs only (client/src/shot_main.rs)."
  exit 2
fi

# ---- stale-binary gate ---------------------------------------------------
newest_src=0; newest_name=""
for f in $SRC; do t=$(mt "$f"); if [ "$t" -gt "$newest_src" ]; then newest_src=$t; newest_name=$f; fi; done
exe_t=$(mt "$EXE")
echo "$(stamp) exe   : $EXE"
echo "$(stamp)         mtime $(date -d @"$exe_t" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE") bytes"
echo "$(stamp) source: newest edited $newest_name  $(date -d @"$newest_src" '+%Y-%m-%d %H:%M:%S')"
if [ "$exe_t" -lt "$newest_src" ]; then
  echo "REFUSED  the exe is OLDER than $newest_name — it cannot contain this pass."
  echo "         Shooting it would produce a before/after of two different builds."
  exit 2
fi
echo "$(stamp) stale-binary gate: PASS"
echo

# ---- shared capture env (identical to the shipped v3 recipe) -------------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0

shoot() { # $1=scene $2=gen $3=tag
  local scene="$1" gen="$2" tag="$3"
  local png="$OUT/${scene}_${tag}.png"
  rm -f "$png"
  VOXELFORGE_LOOK_GEN="$gen" VOXELFORGE_SHOT="$png" \
    "./$EXE" --play > "$OUT/${scene}_${tag}.log" 2>&1
  if [ -f "$png" ]; then
    echo "$(stamp) shot  $scene gen=$gen -> $(basename "$png")  $(stat -c %s "$png") bytes"
  else
    echo "$(stamp) FAILED $scene gen=$gen — no frame written (see ${scene}_${tag}.log)"
  fi
}

# ---- scene 1: outdoor noon ----------------------------------------------
export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
export VOXELFORGE_LOOK_SUN=66,205,20000
export VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
export VOXELFORGE_LOOK_EXPOSURE=10.6
shoot outdoor-noon v3 before
shoot outdoor-noon v4 after
shoot outdoor-noon v4 afterNULL     # null pair: identical env + identical gen
unset VOXELFORGE_LOOK_SUN VOXELFORGE_LOOK_LIGHT VOXELFORGE_LOOK_EXPOSURE

# ---- scene 2: raking evening sun (the shipped GOLDEN hour) --------------
export VOXELFORGE_CINE="18,9,44, 18,9,44, 31,1.5,29, 1"
shoot evening-raking v3 before
shoot evening-raking v4 after

# ---- scene 3: night, lit by fire ----------------------------------------
export VOXELFORGE_CINE="36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1"
export VOXELFORGE_LOOK_NIGHT=1
shoot night-firelit v3 before
shoot night-firelit v4 after
unset VOXELFORGE_LOOK_NIGHT

# ---- de-HUD ---------------------------------------------------------------
# VOXELFORGE_NOHUD=1 hides the Node UI, but nohud2_guard.py names exactly two
# honest producers of a `-nohud2.png` and this is not one of them, so every raw
# capture goes through the de-HUD pass regardless. It is also what the SHIPPED
# baseline plates in _fl_lookv4/ went through — running it here keeps the
# comparison like-for-like instead of comparing a patched frame to an unpatched
# one. On an already-clean frame it removes ~0 px and says so.
echo
echo "$(stamp) de-HUD pass"
python scripts/_flamingo_dehud2.py "$OUT"/*_before.png "$OUT"/*_after.png "$OUT"/*_afterNULL.png

echo
echo "$(stamp) shot set complete -> $OUT"
