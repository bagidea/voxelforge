#!/usr/bin/env bash
# ===========================================================================
# Flamingo — PILE A (client/src/hero.rs) before/after, ONE BINARY.
#
# The other half of the v4 pass. The three play plates
# (scripts/_fl_v4_shoot_20260818.sh) exercise look.rs's V3→V4 grade; NOTHING in
# them exercises Pile A, because Pile A is the interior hero scene and only
# `voxelforge_shot.exe` (client/src/shot_main.rs → hero.rs) renders it.
#
# ONE BINARY, TWO FRAMES — and the "before" is not an old exe, it is the NEW exe
# driven back to the old numbers through the sweep hooks Pile A hoisted for
# exactly this purpose. Every changed const has an env key; the ones this script
# does not set are the ones the commit did not change in value. `BLUESCALE=0.85`
# is documented in `hero.rs` as reproducing the old sun+fog exactly, and the rim
# card is returned to "did not exist" by giving it 0 lux.
#
# The frames are named `-nohud2.png` because `voxelforge_shot.exe` draws hero.rs
# and nothing else — no HUD exists in these pixels (scripts/render_grade.sh
# header, scripts/nohud2_guard.py).
#
# USAGE  scripts/_fl_v4_heroshot_20260818.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
OUT="$PWD/_fl_v4_20260818/hero"
mkdir -p "$OUT"

stamp() { date "+[%H:%M:%S]"; }
mt() { stat -c %Y "$1" 2>/dev/null || echo 0; }

EXE="${1:-target-pixel/release/voxelforge_shot.exe}"
[ -f "$EXE" ] || { echo "REFUSED  no exe at $EXE"; exit 2; }

src_t=$(mt client/src/hero.rs); exe_t=$(mt "$EXE")
echo "$(stamp) exe    $EXE  mtime $(date -d @"$exe_t" '+%Y-%m-%d %H:%M:%S')  size $(stat -c %s "$EXE")"
echo "$(stamp) hero.rs mtime $(date -d @"$src_t" '+%Y-%m-%d %H:%M:%S')"
if [ "$exe_t" -lt "$src_t" ]; then
  echo "REFUSED  exe is older than hero.rs — it cannot contain Pile A."
  exit 2
fi
echo "$(stamp) stale-binary gate: PASS"

shoot() { # $1 = tag
  local png="$OUT/hero-pileA_$1-nohud2.png"
  rm -f "$png"
  VOXELFORGE_SHOT="$png" "./$EXE" > "$OUT/hero-pileA_$1.log" 2>&1
  if [ -f "$png" ]; then
    echo "$(stamp) shot $1 -> $(basename "$png")  $(stat -c %s "$png") bytes"
  else
    echo "$(stamp) FAILED $1 — no frame (see hero-pileA_$1.log)"
  fi
}

# ---- BEFORE: the new exe driven back to the pre-Pile-A constants ----------
(
  export VOXELFORGE_AMBIENT=2800
  export VOXELFORGE_AMBCOLOR=0.70,0.60,0.44
  export VOXELFORGE_BLUESCALE=0.85
  export VOXELFORGE_EXPOSURE=9.0
  export VOXELFORGE_GRADE=0.02,1.00,1.30
  export VOXELFORGE_SHOULDER=0.64
  export VOXELFORGE_BOUNCE2=1.7
  export VOXELFORGE_BOUNCE2COLOR=1.0,0.75,0.42
  export VOXELFORGE_RIM=0.46,0.62,1.0,0        # 0 lux = the card did not exist
  export VOXELFORGE_PANEHI=2.3,1.85,1.25
  export VOXELFORGE_CLEAR=0.05,0.03,0.02
  shoot before
)
# ---- AFTER: the shipped defaults, no env at all ---------------------------
shoot after
# ---- NULL: shipped defaults again — the capture noise floor ---------------
shoot afterNULL

echo
echo "$(stamp) grading (both calibrated instruments)"
python scripts/_fl_v4_grade_20260818.py measure \
  "hero_pileA_before=$OUT/hero-pileA_before-nohud2.png" \
  "hero_pileA_after=$OUT/hero-pileA_after-nohud2.png" \
  --out hero-pileA
echo
echo "---- look-acceptance gate (grade_axes, hero profile) ----"
python scripts/grade_axes.py --profile hero \
  "$OUT/hero-pileA_before-nohud2.png" "$OUT/hero-pileA_after-nohud2.png"
