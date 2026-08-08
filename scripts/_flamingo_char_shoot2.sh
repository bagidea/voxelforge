#!/usr/bin/env bash
# Flamingo — CHARACTER shots, pass 2. Same recipe as _flamingo_char_shoot.sh (baked
# golden hour, real post stack, only VOXELFORGE_LOOK_{CAM,QUALITY} set) with two
# things fixed:
#
#   1. THE POSE IS REAL NOW. `place_player` (scene.rs) used to overwrite orbit.yaw
#      and orbit.pitch right after spawn, so of the three numbers in `_CAM` only
#      DIST survived and pass 1 had to write yaw/pitch to match what the game reset
#      to. That reset now yields to the override, so this script can actually aim.
#
#   2. IT REFUSES TO GRADE A FRAME WHOSE BOOM WAS SHORTENED. `camera_boom` cuts
#      `dist` down to whatever the collision sweep allows; at the campsite the walls
#      are close, so a 5 m boom rendered at a fraction of that and pass 1's mask —
#      built at 5.0 — sampled a slice of the body plus wall. Every candidate pose
#      below is measured by _flamingo_char_dist_solve.py, and only a pose whose body
#      renders at the size its `--cam` implies gets graded.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=_fl_char2
EXE="target-flamingo/release/voxelforge.exe"
mkdir -p "$OUT"
[ -f "$EXE" ] || { echo "no exe at $EXE — build first"; exit 1; }

# Copy before running: running the linker's own output holds a lock on it and the
# next lane's build then dies on access-denied (LANES.md).
PROBE="$OUT/vfchar.exe"
cp -f "$EXE" "$PROBE" || exit 1
echo "exe mtime $(date -r "$EXE" -Is)  size $(stat -c %s "$EXE")"
echo

# name             yaw   pitch   dist
CANDIDATES="
med-p14            0    -14.32  5.0
med-p25            0    -25.0   5.0
med-p35            0    -35.0   5.0
med-p25-y180     180    -25.0   5.0
med-p35-y90       90    -35.0   5.0
close-p14          0    -14.32  2.2
close-p25          0    -25.0   2.2
close-p25-y180   180    -25.0   2.2
"

shoot() {  # name yaw pitch dist
  local name="$1" yaw="$2" pitch="$3" dist="$4"
  local stem="$OUT/$name"
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_CAM="$yaw,$pitch,$dist" \
      VOXELFORGE_LOOK_QUALITY=ultra \
      VOXELFORGE_SHOT="$stem.png" "$PROBE" >"$stem.log" 2>&1
  python scripts/_flamingo_dehud2.py "$stem.png" >/dev/null 2>&1
}

echo "=== SWEEP: shoot every candidate, then measure whether its boom was clear ==="
printf '  %-16s %-18s %8s %10s  %s\n' pose cam ratio implied verdict
BEST_MED=""; BEST_MED_R=99; BEST_CLOSE=""; BEST_CLOSE_R=99
while read -r name yaw pitch dist; do
  [ -z "${name:-}" ] && continue
  shoot "$name" "$yaw" "$pitch" "$dist"
  cam="$yaw,$pitch,$dist"
  out="$(python scripts/_flamingo_char_dist_solve.py --cam "$cam" --quiet \
         "$OUT/$name-nohud2.png" 2>/dev/null)"
  r="$(echo "$out"  | sed -n 's/^RATIO=//p')"
  im="$(echo "$out" | sed -n 's/^IMPLIED_DIST=//p')"
  [ -z "$r" ] && r=nan
  dev="$(python -c "
import sys
try: print(abs(float('$r')-1.0))
except Exception: print(99)
")"
  verdict="$(python -c "print('CLEAR' if $dev <= 0.12 else 'shortened')")"
  printf '  %-16s %-18s %8s %10s  %s\n' "$name" "$cam" "$r" "${im:-nan}" "$verdict"
  case "$name" in
    med-*)   better="$(python -c "print(1 if $dev < $BEST_MED_R else 0)")"
             [ "$better" = 1 ] && { BEST_MED="$name|$cam"; BEST_MED_R="$dev"; } ;;
    close-*) better="$(python -c "print(1 if $dev < $BEST_CLOSE_R else 0)")"
             [ "$better" = 1 ] && { BEST_CLOSE="$name|$cam"; BEST_CLOSE_R="$dev"; } ;;
  esac
done <<< "$CANDIDATES"

echo
echo "=== GRADE (character region only, vs the approved concept sheet) ==="
for pick in "$BEST_MED" "$BEST_CLOSE"; do
  [ -z "$pick" ] && continue
  name="${pick%%|*}"; cam="${pick##*|}"
  echo "--- picked $name  cam=$cam ---"
  python scripts/grade_character.py \
      --cam "$cam" --actor player \
      --ref-json "_fl_char/ref-auren-hero.json" \
      --label "$name" --json "$OUT/$name.json" \
      "$OUT/$name-nohud2.png"
  echo
done
echo "=== DONE $(date -Is) ==="
