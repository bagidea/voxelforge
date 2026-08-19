#!/usr/bin/env bash
# _pixel_pilea_shoot.sh -- BEFORE/AFTER plates for Pile A (grade + lighting),
# docs/art-gap-vs-reference-2026-08-17.md targets 1-4. Pixel lane.
#
# ONE BINARY, EVERY PLATE. The BEFORE side is not an archived file and not a
# second build -- it is this same exe with the pre-Pile-A recipe restored through
# env, value for value. That is the only before/after this repo accepts: two
# binaries 30 bytes apart in 80.7 MB once differed by link stamps alone, so a
# pair shot from two builds cannot separate "the grade changed" from "something
# else did".
#
# The restore list below is the OLD `mod recipe`, verbatim:
#   EXPOSURE 9.0 | GRADE 0.02,1.00,1.30 | SHOULDER 0.64 | AMBIENT 2800
#   AMBCOLOR 0.70,0.60,0.44 (amber) | BLUESCALE 0.85 | BOUNCE2 1.7
#   BOUNCE2COLOR 1.0,0.75,0.42 (warm) | RIM off | PANEHI warm | CLEAR warm-black
# Those same values are also, byte for byte, the env `scripts/_pixel_beauty_render.sh`
# exported to shoot `_fl_beauty_20260816/beauty-wide-nohud2.png` -- which is what
# makes `before-wide` checkable against the archived plate rather than merely
# plausible. `_pixel_bake_diff.py` prints that check at the end, against the r2
# repeat's own capture-noise floor. Without a floor, "differs by N px" has no zero.
#
# AFTER is NO env at all. Not one look knob. Pile A has to stand on the bake, or
# it is the same "approved look lives only in a shell script" bug `mod recipe`
# was written to kill.
set -u
cd "$(dirname "$0")/.."

EXE="${EXE:-target-pixel/release/voxelforge_shot.exe}"
OUT="${OUT:-_pixel_pilea_20260818}"
[[ -f "$EXE" ]] || {
  echo "NO EXE $EXE -- run: .\\scripts\\lane-build.ps1 -Lane pixel -Profile release -Bin voxelforge_shot"
  exit 2
}
mkdir -p "$OUT"

echo "EXE   $EXE"
echo "size  $(stat -c%s "$EXE")b"
echo "mtime $(stat -c%y "$EXE")"
echo "md5   $(md5sum "$EXE" | cut -d' ' -f1)"
echo "rev   $(git rev-parse --short HEAD) (+ uncommitted hero.rs -- this is a working-tree build)"
echo

# The pre-Pile-A recipe, as env. Keep in sync with the `git show HEAD:client/src/hero.rs`
# values of `mod recipe`; every entry here is a const this change moved.
BEFORE=(
  VOXELFORGE_EXPOSURE=9.0
  VOXELFORGE_GRADE=0.02,1.00,1.30
  VOXELFORGE_SHOULDER=0.64
  VOXELFORGE_AMBIENT=2800
  VOXELFORGE_AMBCOLOR=0.70,0.60,0.44
  VOXELFORGE_BLUESCALE=0.85
  VOXELFORGE_BOUNCE2=1.7
  VOXELFORGE_BOUNCE2COLOR=1.0,0.75,0.42
  VOXELFORGE_RIM=0,0,0,0
  VOXELFORGE_PANEHI=2.3,1.85,1.25
  VOXELFORGE_PANEHIBASE=0.98,0.84,0.60
  VOXELFORGE_CLEAR=0.05,0.03,0.02
)

shoot() {
  local name="$1"
  shift
  local out="$OUT/$name-nohud2.png"
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"$OUT/$name.runlog" 2>&1
  local code=$?
  if [[ -f "$out" ]]; then
    # Echo hero.rs's own PILEA line for THIS plate. A frame captioned "after"
    # whose runlog says amb=[0.7,0.6,0.44] is caught here, not by a reviewer
    # squinting at the pixels.
    echo "OK   $out ($(stat -c%s "$out")b) exit=$code"
    grep -hE '^PILEA ' "$OUT/$name.runlog" | sed 's/^/       /'
  else
    echo "MISS $out exit=$code"
    return 1
  fi
}

# ---- scene 1: the WIDE establishing beauty plate --------------------------
# The plate the art-gap report's "OURS interior" column is measured on.
shoot before-wide "${BEFORE[@]}"
shoot before-wide-r2 "${BEFORE[@]}"
shoot after-wide
shoot after-wide-r2

# ---- scene 2: the g4 lever family -----------------------------------------
# `_fl_g2g4_20260816/g4-only-nohud2.png` is the report's second interior plate
# (it measures identically to beauty, which is what makes "OURS interior" one
# look and not a noisy sample). Same G2_PANE=block lever it was shot with, so
# the only thing moving between these two is the grade.
shoot before-g4 "${BEFORE[@]}" VOXELFORGE_G2_PANE=block
shoot after-g4 VOXELFORGE_G2_PANE=block

echo
echo "== capture noise floor + reproduction check =="
python scripts/_pixel_bake_diff.py \
  "FLOOR-before=$OUT/before-wide-nohud2.png:$OUT/before-wide-r2-nohud2.png" \
  "FLOOR-after=$OUT/after-wide-nohud2.png:$OUT/after-wide-r2-nohud2.png" \
  "REPRO-before-vs-archived=$OUT/before-wide-nohud2.png:_fl_beauty_20260816/beauty-wide-nohud2.png" \
  "before-vs-after=$OUT/before-wide-nohud2.png:$OUT/after-wide-nohud2.png"

echo
echo "== Pile A gates =="
python scripts/_pixel_pilea_measure.py \
  "before_wide=$OUT/before-wide-nohud2.png" \
  "after_wide=$OUT/after-wide-nohud2.png"
echo "gates exit=$?"

echo DONE
