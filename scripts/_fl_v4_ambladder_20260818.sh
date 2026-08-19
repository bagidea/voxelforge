#!/usr/bin/env bash
# ===========================================================================
# Flamingo — PILE A follow-up: WHICH light turned the room blue?
#
# The committed Pile A frame moves five axes toward REF and then overshoots the
# one it was aimed at: warm 97.1 % -> 10.3 % (REF 65.2), cool 0.0 % -> 63.1 %
# (REF 16.9). Looking at the plate, the room is not warm-key + cool-sky; it is
# uniformly blue.
#
# HYPOTHESIS. `AMBCOLOR` is the colour of the FLAT AMBIENT FILL, and a flat fill
# is by definition omnidirectional — it reaches the sunlit island top exactly as
# much as the -Z cabinet fronts. So tinting it sky-blue AND raising it (2800 ->
# 3900 lux) cannot put cool "where the window reaches"; it repaints every face in
# frame. The one light in the rig that IS selective is the new RIM card, which is
# aimed down and inward.
#
# If that is right, then pulling AMBIENT down while pushing RIM up should trade
# cool-everywhere for cool-where-it-belongs, and land warm/cool near REF without
# giving back the brightness Pile A bought.
#
# This is a SWEEP, not a rebuild: every lever Pile A moved has an env key, so all
# rungs come from ONE binary. Rung 0 is the committed default with no env at all,
# so the ladder contains its own control.
#
# EVERY RUNG IS SHOT AND MEASURED. No rung is dropped for being ugly — a ladder
# that only reports its winner is a ladder you cannot check.
#
# USAGE  scripts/_fl_v4_ambladder_20260818.sh [exe]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
OUT="$PWD/_fl_v4_20260818/ambladder"
mkdir -p "$OUT"
EXE="${1:-target-pixel/release/voxelforge_shot.exe}"
[ -f "$EXE" ] || { echo "REFUSED  no exe at $EXE"; exit 2; }
stamp() { date "+[%H:%M:%S]"; }

echo "$(stamp) exe $EXE  $(stat -c %s "$EXE") bytes  mtime $(date -d @"$(stat -c %Y "$EXE")" '+%H:%M:%S')"

# rung | AMBIENT | AMBCOLOR | RIM(lux)
RUNGS="
r0-committed|3900|0.30,0.45,0.80|5200
r1-amb2400|2400|0.30,0.45,0.80|5200
r2-amb2400-rim8k|2400|0.30,0.45,0.80|8000
r3-amb1800-rim8k|1800|0.30,0.45,0.80|8000
r4-paleblue|3900|0.55,0.62,0.80|5200
r5-amb2400-pale-rim8k|2400|0.55,0.62,0.80|8000
"

ARGS=""
for row in $RUNGS; do
  [ -z "$row" ] && continue
  name=$(echo "$row" | cut -d'|' -f1)
  amb=$(echo  "$row" | cut -d'|' -f2)
  col=$(echo  "$row" | cut -d'|' -f3)
  rim=$(echo  "$row" | cut -d'|' -f4)
  png="$OUT/${name}-nohud2.png"
  rm -f "$png"
  if [ "$name" = "r0-committed" ]; then
    # the shipped default, driven by NO env at all — the ladder's own control
    VOXELFORGE_SHOT="$png" "./$EXE" > "$OUT/$name.log" 2>&1
  else
    VOXELFORGE_AMBIENT="$amb" VOXELFORGE_AMBCOLOR="$col" \
      VOXELFORGE_RIM="0.46,0.62,1.0,$rim" \
      VOXELFORGE_SHOT="$png" "./$EXE" > "$OUT/$name.log" 2>&1
  fi
  if [ -f "$png" ]; then
    echo "$(stamp) rung $name  AMBIENT=$amb AMBCOLOR=$col RIM_lux=$rim -> $(stat -c %s "$png") bytes"
    ARGS="$ARGS ${name}=$png"
  else
    echo "$(stamp) rung $name FAILED — no frame (see $name.log)"
  fi
done

echo
echo "$(stamp) measuring every rung (art-gap instrument)"
python scripts/_fl_v4_grade_20260818.py measure $ARGS --out ambladder
echo
echo "$(stamp) look-acceptance gate on every rung (p95 is the one Pile A broke)"
python scripts/grade_axes.py --profile hero "$OUT"/*-nohud2.png 2>&1 | grep -E "^E:|p95|clip|micro|=>"
