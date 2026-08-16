#!/usr/bin/env bash
# _pixel_beauty_render.sh -- re-shoot the golden beauty-shot SET from current source.
#
# Why this exists next to scripts/render_wide_hero.sh instead of editing it:
# render_wide_hero.sh hardcodes target/release and writes straight over the tracked
# canonical docs/assets/wide-hero-final.png, which is ALSO the parity baseline every
# other lane grades against (docs/look-acceptance-rubric.md, "framing baseline" row).
# Overwriting it mid-review is exactly the failure mode logged in memory as
# "pin plates, not paths". So: same recipe, byte-for-byte on the env values, but
# built out of the pixel lane's own target-pixel/ and written to a dated scratch dir.
#
# Two plates, one binary, one source tree:
#   A) beauty-wide  -- the CEO-approved TILT-DOWN establishing hero (recipe verbatim
#      from render_wide_hero.sh). Graded against the framing baseline.
#   B) beauty-tight -- the SAME hero scene with WIDE off, i.e. the framing family the
#      1024x1024 calibration ref was shot on. This is the only plate whose absolute
#      P0 numbers (DOF included) may honestly be compared with the ref's.
#
# Frames are named *-nohud2.png because voxelforge_shot draws no HUD at all; the
# graders refuse any other filename (grade_axes.py's de-HUD guard).
set -u
cd "$(dirname "$0")/.."

EXE="${EXE:-target-pixel/release/voxelforge_shot.exe}"
OUTDIR="${OUTDIR:-_fl_beauty_20260816}"
[[ -f "$EXE" ]] || { echo "NO EXE $EXE -- run: .\\scripts\\lane-build.ps1 -Lane pixel -Profile release -Bin voxelforge_shot"; exit 2; }
mkdir -p "$OUTDIR"

echo "EXE   $EXE  ($(stat -c%s "$EXE")b, mtime $(stat -c%y "$EXE"))"

shoot() {
  local name="$1"; shift
  local out="$OUTDIR/$name-nohud2.png"
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" > "$OUTDIR/$name.runlog" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $name exit=$code (see $OUTDIR/$name.runlog)"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# ---- A) wide establishing hero -- recipe verbatim from render_wide_hero.sh -------
shoot beauty-wide \
  VOXELFORGE_WIDE=1 \
  VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60 \
  VOXELFORGE_DOF=8,10 \
  VOXELFORGE_SUN=19,196,26000 \
  VOXELFORGE_AMBIENT=2800 \
  VOXELFORGE_BLUESCALE=0.85 \
  VOXELFORGE_EXPOSURE=9.0 \
  VOXELFORGE_GRADE=0.02,1.00,1.30 \
  VOXELFORGE_SHOULDER=0.64 \
  VOXELFORGE_DUST=3.0 \
  VOXELFORGE_BOUNCE=1.0 \
  VOXELFORGE_BOUNCE2=1.7 \
  VOXELFORGE_AMBCOLOR=0.70,0.60,0.44

# ---- B) tight hero -- hero.rs baked defaults, no framing override ---------------
# Nothing but the output path is set: this plate is the "does the tree render the
# approved look OUT OF THE BOX" question. Any env knob here would launder a baked
# regression into a pass (memory: "verify default reproduces the approved artifact").
shoot beauty-tight

echo "DONE"
