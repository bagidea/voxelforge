#!/usr/bin/env bash
# Flamingo — re-capture the four graded frames on a binary that actually
# contains the shipped look constants (2026-08-06).
#
# WHY. Every frame in `docs/aaa-gap-scorecard-2026-08-06.md` was shot on a
# binary older than the look values it was used to judge:
#
#   docs/assets/gate3/gate3-after-*.png       2026-08-01 12:14   (5 days stale)
#   docs/assets/wide-hero-final-nohud2.png    2026-08-05 15:31
#   target/release/voxelforge.exe             2026-08-06 06:31
#   client/src/look.rs (HEAD f358809)         2026-08-06 06:48
#
# `bd3cde5` (POST_SATURATION 1.35 -> 1.90) landed 2026-08-05 17:38 and
# `f358809` (HAZE_FULL 250 -> 150) at 06:53 — so the gate3 columns of the
# scorecard measure NEITHER, and the sat/warmth gaps it reports may already be
# closed in source. This script re-shoots them off one freshly linked exe.
#
# THE CONTROL ROW EXISTS ON PURPOSE. This run also changes two constants
# (`Hour::GOLDEN` ambient 1100 -> 2200 lux / B 0.60 -> 0.48, and PCSS_WIDTH
# 3.0 -> 4.0). Shooting only the new default would confound "the 2200 lift
# worked" with "1.90 was already enough" — the exact mistake that produced the
# stale scorecard. So every frame is shot TWICE off the SAME binary: once at
# the new baked default, once with the OLD values restored through the
# `VOXELFORGE_LOOK_*` sweep hooks. The delta between the two columns is the
# patch's real contribution; the CTRL column alone answers Sun's open question
# about POST_SATURATION = 1.90.
#
# Flags are the ones the graded frames used, unchanged:
#   gate3  — VOXELFORGE_PLAY=1 / _PLAY_DEMO=1 / _COMBAT_DEMO=1, QUALITY=high
#   vista  — VOXELFORGE_PLAY=1 + LOOK_CAM=35,-18,26, QUALITY=ultra
#            (scripts/vista_grade_sweep.sh row `v00-ctrl`)
#
# Runs off a COPY of the exe: a shoot that holds target-flamingo's own
# voxelforge.exe open blocks the next relink, and that failure mode has
# already cost this project one false "build green".
set -uo pipefail
cd "$(dirname "$0")/.."

SRC_EXE="${SRC_EXE:-target-flamingo/release/voxelforge.exe}"
OUT="${OUT:-_fl_recap_20260806}"
BIN="$OUT/vfprobe.exe"

[ -s "$SRC_EXE" ] || { echo "NO EXE: $SRC_EXE"; exit 2; }
mkdir -p "$OUT"
cp -f "$SRC_EXE" "$BIN" || { echo "COPY FAILED (exe locked?)"; exit 2; }

echo "=== provenance ==="
echo "  src exe : $SRC_EXE"
echo "  mtime   : $(stat -c '%y' "$SRC_EXE" | cut -d. -f1)"
echo "  sha256  : $(sha256sum "$SRC_EXE" | awk '{print $1}')"
echo "  commit  : $(git rev-parse --short=9 HEAD)  (+ uncommitted look.rs patch)"
echo

# The OLD values, restored through the sweep hooks — see the header.
CTRL_ENV=(
  "VOXELFORGE_LOOK_AMBIENT=1100"
  "VOXELFORGE_LOOK_LIGHT=1.00,0.92,0.62,0.96,0.90,0.60"
  "VOXELFORGE_LOOK_PCSS=3.0"
)

FAILS=0

# $1=stem  $2=arm(new|ctrl)  $3=quality  $4..=mode env KEY=VALUE
shoot() {
  local stem="$1" arm="$2" quality="$3"; shift 3
  local png="$OUT/${stem}-${arm}.png"
  local log="$OUT/${stem}-${arm}.log"
  local -a env_kv=("$@")

  env_kv+=("VOXELFORGE_LOOK_QUALITY=$quality" "VOXELFORGE_SHOT=$png")
  [ "$arm" = "ctrl" ] && env_kv+=("${CTRL_ENV[@]}")

  rm -f "$png"
  env "${env_kv[@]}" "./$BIN" >"$log" 2>&1
  local rc=$?

  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')

  local bad=""
  [ "$rc" -ne 0 ]                  && bad="$bad exit=$rc"
  [ "$bytes" -lt 2048 ]            && bad="$bad no-png"
  grep -qE 'panicked|B0001' "$log" && bad="$bad PANIC"
  grep -q 'SHOT saved' "$log"      || bad="$bad no-SHOT"

  if [ -n "$bad" ]; then
    echo "  X $stem/$arm FAIL:$bad"
    grep -nE 'panicked|B0001|error\[' "$log" | head -3
    FAILS=$((FAILS + 1))
    return 1
  fi

  # Every gate number in this repo is only valid on a de-HUDded frame; a --play
  # capture burns the "[E]" prompt onto the ground at ~250, which false-FAILs G5
  # and drags p95 to the UI. So this is not optional post-processing.
  python scripts/_flamingo_dehud2.py "$png" >/dev/null 2>&1
  local nohud="${png%.png}-nohud2.png"
  [ -s "$nohud" ] || { echo "  X $stem/$arm dehud produced nothing"; FAILS=$((FAILS + 1)); return 1; }
  echo "  OK $stem/$arm  ->  $nohud  (${bytes} B raw)"
  return 0
}

for arm in new ctrl; do
  echo "--- arm: $arm ---"
  shoot gate3-boot   "$arm" high  "VOXELFORGE_PLAY=1"
  shoot gate3-walk   "$arm" high  "VOXELFORGE_PLAY_DEMO=1"
  shoot gate3-combat "$arm" high  "VOXELFORGE_COMBAT_DEMO=1"
  shoot grade-vista  "$arm" ultra "VOXELFORGE_PLAY=1" "VOXELFORGE_LOOK_CAM=35,-18,26"
  echo
done

echo "=== recap done — failures: $FAILS / 8 ==="
[ "$FAILS" -eq 0 ] || exit 1
