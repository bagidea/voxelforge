#!/usr/bin/env bash
# Poppy — v6 candidate search: ONE v3 frame per candidate, ONE binary, no relink.
#
# WHY THIS EXISTS AND WHY IT IS NOT A GATE. v6 has to move constants that only
# exist inside `if v3()`, and the honest way to size them is to shoot the real
# frame at several values. Rebuilding per candidate is ~20 min a rung. Every
# lever v6 needs already has an env hook that `hour()` / `rim_lux()` /
# `grade_knobs()` read BEFORE the shipped constant, so the whole search runs on
# the v5 exe and only the WINNER is baked and rebuilt:
#
#   AMBIENT_LUX_V3_NIGHT  -> VOXELFORGE_LOOK_AMBIENT=<lux>
#   Hour::ambient colour  -> VOXELFORGE_LOOK_LIGHT=kr,kg,kb,ar,ag,ab
#   RIM_LUX               -> VOXELFORGE_LOOK_RIM=<lux>
#   grade::TEMPERATURE    -> VOXELFORGE_LOOK_GRADE=temp,sat,mid,hi_gain
#
# THE FIRST ROW OF EVERY SWEEP IS `ctrl`, WITH NO OVERRIDES AT ALL. If ctrl does
# not reproduce the committed `<scene>_after.png` numbers, the hook is not
# reaching the shader (or the shoot is not deterministic) and no other row on the
# sheet means anything. This lane has shipped a wrong root cause off a repro that
# was never a repro; the control is the cheap way not to do it again.
#
# The scene fixtures below are copied from `_poppy_lookv3_shoot.cmd` and must
# stay byte-identical to it -- a sweep shot through a different camera pose or a
# different exposure is measuring a different picture.
#
# Usage: bash scripts/_poppy_lookv6_sweep.sh <scene> <outdir> <label>=<env...> ...
#   e.g. bash scripts/_poppy_lookv6_sweep.sh night-firelit _poppy_lookv6/n1 \
#          ctrl \
#          'a24:VOXELFORGE_LOOK_AMBIENT=24' \
#          'a24w:VOXELFORGE_LOOK_AMBIENT=24;VOXELFORGE_LOOK_LIGHT=0.55,0.66,0.95,0.90,0.72,0.44'
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT=$PWD
EXE=${EXE:-$ROOT/target-poppy/perf/voxelforge.exe}
SCENE=${1:?scene}
OUT=${2:?outdir}
shift 2
mkdir -p "$OUT"

[[ -f "$EXE" ]] || { echo "no exe at $EXE"; exit 2; }
echo "SWEEP: exe    $EXE"
echo "SWEEP: sha256 $(sha256sum "$EXE" | awk '{print $1}')"
echo "SWEEP: scene  $SCENE"

# ---- scene fixtures, verbatim from _poppy_lookv3_shoot.cmd -----------------
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0
export VOXELFORGE_LOOK_GEN=v3

case "$SCENE" in
  outdoor-noon)
    export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
    export VOXELFORGE_LOOK_SUN="66,205,20000"
    export VOXELFORGE_LOOK_LIGHT="1.00,0.98,0.93,0.84,0.88,1.00"
    export VOXELFORGE_LOOK_EXPOSURE="10.6"
    ;;
  evening-raking)
    export VOXELFORGE_CINE="18,9,44, 18,9,44, 31,1.5,29, 1"
    ;;
  night-firelit)
    export VOXELFORGE_CINE="36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1"
    export VOXELFORGE_LOOK_NIGHT=1
    ;;
  *) echo "unknown scene $SCENE"; exit 2 ;;
esac

for spec in "$@"; do
  label=${spec%%:*}
  envs=${spec#*:}
  [[ "$envs" == "$label" ]] && envs=""

  shot="$OUT/${SCENE}__${label}.png"
  echo "[shoot] $label -> $(basename "$shot")   ${envs:-(no overrides)}"

  (
    IFS=';'
    for kv in $envs; do
      [[ -z "$kv" ]] && continue
      export "${kv?}"
    done
    VOXELFORGE_SHOT="$(cygpath -w "$shot")" "$EXE" --play >> "$OUT/_sweep.log" 2>&1
  )

  [[ -f "$shot" ]] || { echo "  MISSING FRAME — $label wrote nothing"; exit 1; }
done

echo "SWEEP: $# frame(s) in $OUT"
