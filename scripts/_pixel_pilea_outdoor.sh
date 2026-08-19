#!/usr/bin/env bash
# _pixel_pilea_outdoor.sh -- port the proven `_flamingo_g6`/`_g7` sky* grade onto
# the SHIPPED `_fl_lookv4` outdoor plates, and measure what it buys. Pixel lane.
#
# WHY THIS IS AN ENV RUN AND NOT A SOURCE BAKE
# The outdoor default lives in `look.rs`'s `Hour::GOLDEN.sky`, and `look.rs` is
# poppy's file (docs/LANES.md, "Look post stack"). LANES.md's rule for exactly this
# case is "if you need a change in another lane to finish your own work, say so and
# stop" -- so this script proves the value on the shipped scenes through the env
# lever `look.rs` already exposes for the purpose (line 1808), and the one-const
# diff is handed to poppy rather than applied here.
#
# ONE BINARY, BOTH SIDES. `before` is the `_poppy_lookv3_shoot.cmd` recipe verbatim
# (which is what produced the archived `_fl_lookv4` set); `after` is the SAME exe,
# SAME camera, SAME hour, SAME sun, with VOXELFORGE_LOOK_SKY the only thing moving.
# The exe is COPIED out of target-poppy before it is run: running poppy's exe in
# place would hold it open and fail their next relink with "Access is denied".
#
# Usage: bash scripts/_pixel_pilea_outdoor.sh [sky_r,sky_g,sky_b ...]
#        (no args = the four candidate rungs below)
set -u
cd "$(dirname "$0")/.."
ROOT=$PWD

SRC_EXE="${SRC_EXE:-target-poppy/perf/voxelforge.exe}"
OUT="${OUT:-_pixel_pilea_outdoor_20260818}"
EXE="$OUT/voxelforge-pinned.exe"

[[ -f "$SRC_EXE" ]] || { echo "NO EXE $SRC_EXE"; exit 2; }
mkdir -p "$OUT"
if [[ ! -f "$EXE" ]] || [[ "$SRC_EXE" -nt "$EXE" ]]; then
  cp -f "$SRC_EXE" "$EXE" || exit 2
fi
echo "SRC   $SRC_EXE ($(stat -c%s "$SRC_EXE")b, mtime $(stat -c%y "$SRC_EXE"))"
echo "PINNED $EXE md5=$(md5sum "$EXE" | cut -d' ' -f1)"
# The lever has to actually EXIST in this binary or every 'after' plate below is
# just a re-shoot of the before under a different filename.
#
# `grep -a` on the exe, NOT `strings | grep` -- there is no `strings` on this box,
# so `strings ... 2>/dev/null | grep -q` is an empty pipe that reports ABSENT for
# every binary ever built. It did, on the first run of this script. A probe whose
# failure mode is "always fail" is as useless as one that always passes, so both
# directions are controlled: a string that MUST be present and one that MUST NOT.
lever_hits=$(grep -ac 'VOXELFORGE_LOOK_SKY' "$EXE" || true)
ctl_pos=$(grep -ac 'VOXELFORGE_LOOK_SUN' "$EXE" || true)          # sibling lever, same fn
ctl_neg=$(grep -ac 'VOXELFORGE_PIXEL_NOT_A_REAL_LEVER' "$EXE" || true)
echo "LEVER  _LOOK_SKY=$lever_hits  (+control _LOOK_SUN=$ctl_pos, -control=$ctl_neg)"
if [[ "$ctl_pos" -eq 0 || "$ctl_neg" -ne 0 ]]; then
  echo "       PROBE BROKEN -- the +control must hit and the -control must not. Refusing."
  exit 2
fi
if [[ "$lever_hits" -eq 0 ]]; then
  echo "       ABSENT -- this exe predates look.rs:1808; refusing to shoot"
  exit 2
fi
echo

# Rungs. The first three are values the g6/g7 sweep already published a cool% for
# (13.4 / 13.6 / 19.1); the fourth is this lane's own, brighter still.
RUNGS=("$@")
if [[ ${#RUNGS[@]} -eq 0 ]]; then
  RUNGS=(0.586,0.981,1.435 0.54,0.90,1.35 0.90,1.50,2.25 1.05,1.65,2.45)
fi

# `_poppy_lookv3_shoot.cmd` scene 1, verbatim. v3 is the shipped generation, so the
# BEFORE here is the archived `_fl_lookv4/outdoor-noon-after-nohud2.png` recipe --
# "after" in that pair meant "after the v3 rig", not "after this change".
shoot() { # name  [extra env...]
  local name="$1"; shift
  local out="$OUT/$name-nohud2.png"
  rm -f "$out"
  env VOXELFORGE_PLAY=1 VOXELFORGE_NOHUD=1 VOXELFORGE_LOOK_QUALITY=ultra \
      VOXELFORGE_CINE_START=1.0 \
      VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1" \
      VOXELFORGE_LOOK_SUN=66,205,20000 \
      VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00 \
      VOXELFORGE_LOOK_EXPOSURE=10.6 \
      VOXELFORGE_LOOK_GEN=v3 \
      "$@" \
      VOXELFORGE_SHOT="$ROOT/$out" "$EXE" --play >"$OUT/$name.runlog" 2>&1
  local code=$?
  if [[ -f "$out" ]]; then
    echo "OK   $out ($(stat -c%s "$out")b) exit=$code"
  else
    echo "MISS $out exit=$code (tail: $(tail -2 "$OUT/$name.runlog" | tr '\n' ' '))"
    return 1
  fi
}

shoot outdoor-noon-before
i=0
for sky in "${RUNGS[@]}"; do
  i=$((i + 1))
  echo "rung $i: VOXELFORGE_LOOK_SKY=$sky"
  shoot "outdoor-noon-sky$i" VOXELFORGE_LOOK_SKY="$sky"
done

echo
echo "== Pile A gates, outdoor =="
args=("archived=_fl_lookv4/outdoor-noon-after-nohud2.png"
      "before=$OUT/outdoor-noon-before-nohud2.png")
i=0
for sky in "${RUNGS[@]}"; do
  i=$((i + 1))
  args+=("sky$i=$OUT/outdoor-noon-sky$i-nohud2.png")
done
python scripts/_pixel_pilea_measure.py "${args[@]}"

echo DONE
