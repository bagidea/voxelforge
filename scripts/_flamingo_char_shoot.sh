#!/usr/bin/env bash
# Flamingo — CHARACTER shots (close-up + medium) under the BAKED golden hour,
# through the real post stack, then graded on the character region only.
#
# Recipe is verify_baked_grade.sh's, unchanged in the part that matters: the
# only VOXELFORGE_LOOK_* vars set are `_CAM` and `_QUALITY`. Not one grade,
# light, haze or exposure value is overridden, so what is measured here is the
# character standing in the hour the binary actually ships (`Hour::GOLDEN`,
# look.rs) — not a character lit by an env row.
#
# ONE HONEST CAVEAT ABOUT `_CAM`. In `--play`, scene.rs::place_player runs at
# boot and overwrites `orbit.yaw` (-> camp.yaw) and `orbit.pitch` (-> WAKE_PITCH
# = -0.25 rad = -14.32 deg). It does NOT touch `want_dist`, which is the field
# `camera_boom` reads. So of the three numbers in `_CAM`, only DIST survives to
# the shot. The yaw/pitch passed below are therefore written to MATCH the values
# the game resets to, so the string is a true record of the pose rather than a
# request that was silently ignored — and `grade_character.py --cam` is handed
# the same three numbers, which is what makes its bbox land on the body.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=_fl_char
WT=_fl_char_wt
EXE="target-flamingo/release/voxelforge.exe"
PITCH=-14.32
YAW=0
mkdir -p "$OUT"

if [ ! -f "$EXE" ]; then
  echo "no exe at $EXE — build first"; exit 1
fi

# Copy before running: running the linker's own output holds a lock on it and
# the next lane's build then dies on access-denied (LANES.md).
PROBE="$OUT/vfchar.exe"
cp -f "$EXE" "$PROBE" || exit 1
echo "exe mtime $(date -r "$EXE" -Is)  size $(stat -c %s "$EXE")"

shoot() {  # name dist
  local name="$1" dist="$2"
  local stem="$OUT/$name"
  echo "=== SHOOT $name  dist=$dist ==="
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_CAM="$YAW,$PITCH,$dist" \
      VOXELFORGE_LOOK_QUALITY=ultra \
      VOXELFORGE_SHOT="$stem.png" "$PROBE" >"$stem.log" 2>&1
  echo "  exit=$?  bytes=$(stat -c %s "$stem.png" 2>/dev/null || echo 0)"
  python scripts/_flamingo_dehud2.py "$stem.png" >/dev/null 2>&1
  echo "  nohud2 bytes=$(stat -c %s "${stem}-nohud2.png" 2>/dev/null || echo 0)"
}

shoot char-medium  5.0
shoot char-closeup 2.2

echo
echo "=== GRADE (character region only, vs the approved concept sheet) ==="
for pair in "char-medium 5.0" "char-closeup 2.2"; do
  set -- $pair
  python scripts/grade_character.py \
      --cam "$YAW,$PITCH,$2" --actor player \
      --ref-json "$OUT/ref-auren-hero.json" \
      --label "$1" --json "$OUT/$1.json" \
      "$OUT/$1-nohud2.png"
  echo
done
echo "=== DONE $(date -Is) ==="
