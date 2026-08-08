#!/usr/bin/env bash
# Flamingo — P0 chromatic-axis sweep on ONE release binary (2026-08-05).
#
# The gate3 PNGs on disk were shot 2026-08-01, before the tonemapper swap
# (AcesFitted -> TonyMcMapface, POST_SATURATION 1.02 -> 1.05) and before the
# G7/G7b haze + sky_gain work. They are not the current build's numbers. This
# script re-shoots the SAME framings off the exe that is on disk now and sweeps
# the two levers the 2026-08-01 review measured (POST_SATURATION and the
# key/ambient light colours) through the existing env hooks, so the whole search
# runs on one binary instead of a rebuild per candidate.
#
# Every frame goes through scripts/_flamingo_dehud2.py before grading — the
# graders refuse a non-de-HUDded frame, and rightly so.
#
# Usage: bash scripts/_flamingo_grade_sweep.sh [FRAMING] [label ...]
#   FRAMING = boot | walk | combat | vista   (default boot)
set -uo pipefail
cd "$(dirname "$0")/.."

BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-_fl_grade_sweep}"
FRAMING="${1:-boot}"; shift || true
mkdir -p "$OUT"
[ -f "$BIN" ] || { echo "NO EXE $BIN"; exit 2; }

case "$FRAMING" in
  boot)   MODE_ENV=(VOXELFORGE_PLAY=1) ;;
  walk)   MODE_ENV=(VOXELFORGE_PLAY_DEMO=1) ;;
  combat) MODE_ENV=(VOXELFORGE_COMBAT_DEMO=1) ;;
  vista)  MODE_ENV=(VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_CAM=35,-18,26) ;;
  *) echo "unknown framing $FRAMING"; exit 2 ;;
esac

# label       GRADE temp,sat,mid,hi_gain        LIGHT keyRGB,ambRGB
ROWS=(
  "r00-ctrl   0.02,1.05,1.12,0.86   1.00,0.84,0.62,0.96,0.84,0.66"
  "r01-s150   0.02,1.50,1.12,0.86   1.00,0.84,0.62,0.96,0.84,0.66"
  "r02-s190   0.02,1.90,1.12,0.86   1.00,0.84,0.62,0.96,0.84,0.66"
  "r03-s230   0.02,2.30,1.12,0.86   1.00,0.84,0.62,0.96,0.84,0.66"
  "r04-s270   0.02,2.70,1.12,0.86   1.00,0.84,0.62,0.96,0.84,0.66"
  "r05-s190L  0.02,1.90,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r06-s230L  0.02,2.30,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r07-s270L  0.02,2.70,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r08-s310L  0.02,3.10,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r10-s120L  0.02,1.20,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r11-s140L  0.02,1.40,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r12-s160L  0.02,1.60,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  "r13-s150M  0.02,1.50,1.12,0.86   1.00,0.88,0.57,0.97,0.84,0.52"
  "r14-s180M  0.02,1.80,1.12,0.86   1.00,0.88,0.57,0.97,0.84,0.52"
  "r15-s140Lb 0.02,1.40,1.12,0.86   1.00,0.92,0.52,0.98,0.84,0.38"
  # Round 3 -- G-LIFT lights. The shipped hues violate the 2026-08-01 safety rule
  # (key G-B 0.22, ambient G-B 0.18, both under the >=0.30 floor), which is why
  # saturating them grows magenta; the 2026-08-01 prescription fixes that by
  # DRAINING B, which on this pale-stone ruin reads mustard (r06/r15 by eye).
  # These rows hold B where it shipped and LIFT G instead, and walk the ambient
  # lux down so the frame is key-lit rather than fill-lit.
  "r20-s145G  0.02,1.45,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
  "r21-s175G  0.02,1.75,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
  "r22-s175Ga 0.02,1.75,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   VOXELFORGE_LOOK_AMBIENT=700"
  "r23-s175Gb 0.02,1.75,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   VOXELFORGE_LOOK_AMBIENT=450"
  "r24-s205Ga 0.02,2.05,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   VOXELFORGE_LOOK_AMBIENT=700"
  "r21b-s175G 0.02,1.75,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
  # Round 4 -- minimal fix: G-lift lights at or near the SHIPPED saturation.
  # Rounds 1-3 proved the chroma push that clears the P0 axes also turns the
  # ruin mustard and the mauve slab violet; these rows keep the picture and
  # only pull the lights back inside the 2026-08-01 safety envelope.
  "r30-s105G  0.02,1.05,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
  "r31-s120G  0.02,1.20,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
  "r32-s135G  0.02,1.35,1.12,0.86   1.00,0.92,0.62,0.96,0.90,0.60   -"
)

WANT=("$@")
run_row() {
  local label=$1 grade=$2 light=$3 extra=${4:--}
  local stem="$OUT/${FRAMING}-${label}"
  local -a EX=(); [ "$extra" != "-" ] && EX=("$extra")
  echo "--- $label  GRADE=$grade  LIGHT=$light  EXTRA=$extra"
  env "${MODE_ENV[@]}" "${EX[@]}" VOXELFORGE_LOOK_QUALITY=high \
      VOXELFORGE_LOOK_GRADE="$grade" VOXELFORGE_LOOK_LIGHT="$light" \
      VOXELFORGE_SHOT="$stem.png" "$BIN" >"$stem.log" 2>&1
  local rc=$?
  if [ ! -s "$stem.png" ]; then echo "  MISS (exit=$rc)"; tail -3 "$stem.log"; return 1; fi
  grep -qE 'panicked|B0001' "$stem.log" && echo "  !! PANIC"
  python scripts/_flamingo_dehud2.py "$stem.png" >/dev/null 2>&1
  echo "  ok exit=$rc -> ${stem}-nohud2.png"
}

for row in "${ROWS[@]}"; do
  set -- $row
  label=$1 grade=$2 light=$3 extra=${4:--}
  if [ ${#WANT[@]} -gt 0 ]; then
    hit=0; for w in "${WANT[@]}"; do [ "$w" = "$label" ] && hit=1; done
    [ $hit -eq 1 ] || continue
  fi
  run_row "$label" "$grade" "$light" "$extra"
done
echo "=== sweep done ($FRAMING) ==="
