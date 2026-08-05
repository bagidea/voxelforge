#!/usr/bin/env bash
# Flamingo — P0 chromatic sweep on the VISTA framing (2026-08-05, round 5).
#
# WHY A SECOND SWEEP. `scripts/_flamingo_grade_sweep.sh` walked the same two
# levers (POST_SATURATION, key/ambient hue) on the BOOT framing. The numbers the
# review is actually failing on are the vista ones — 41.5 / 44.8 / 60.6 / 1.82
# off `_flamingo_look_audit/look-ultra-vista-nohud2.png` — and
# `scripts/band_map.py` shows why the two framings answer differently:
#
#   band row      meanRGB          R-B
#   y   0- 384    ~110, 89, 85     ~26   <- sky + HAZED distant geometry
#   y 640-1024    ~ 64,107, 17     ~46   <- near grass, already near target
#
# The vista band is 28% washed-out atmosphere and 72% vegetation. Blue 44.8 is
# almost entirely the top third; saturation 60.6 is dragged down by the same
# pixels (near grass measures ~86% on its own). So on THIS framing the haze is a
# bigger chromatic lever than POST_SATURATION, and the boot sweep never touched
# it. These rows walk haze colour/gain against saturation to find out which.
#
# Runs on a COPY of the release exe (`_fl_grade2/vfprobe.exe`) on purpose: a
# sweep that runs `target/release/voxelforge.exe` holds a lock on the file
# another lane's linker is trying to write, and that failure mode has already
# cost this project one false "build เขียว".
#
# Usage: bash scripts/vista_grade_sweep.sh [label ...]
set -uo pipefail
cd "$(dirname "$0")/.."

BIN="${BIN:-_fl_grade2/vfprobe.exe}"
OUT="${OUT:-_fl_grade2}"
mkdir -p "$OUT"
[ -f "$BIN" ] || { echo "NO EXE $BIN"; exit 2; }

# label      GRADE temp,sat,mid,hi_gain   LIGHT kr,kg,kb,ar,ag,ab   EXTRA env (- = none)
ROWS=(
  # v00 is the CONTROL: no grade/light/haze override at all, so it must
  # reproduce the audit frame's 41.47 / 44.84 / 60.64. If it does not, the
  # harness is lying and every row below it is worthless.
  "v00-ctrl      -                     -                              -"
  # The working tree's uncommitted default (sat 1.35 + G-lift lights), so the
  # sweep is measured against what would ship today, not against the 15:12 exe.
  "v01-tree      0.02,1.35,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  -"
  # --- haze alone, tree grade held ---------------------------------------
  "v02-hzwarm1   0.02,1.35,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.78,0.66,0.46,0.62"
  "v03-hzwarm2   0.02,1.35,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  # thinner air: keep the hue, drop how much of it is mixed in at all
  "v04-hzthin    0.02,1.35,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZE=0.0040"
  # --- haze + saturation together ----------------------------------------
  "v05-hz2s175   0.02,1.75,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  "v06-hz2s205   0.02,2.05,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  # --- saturation alone, for the isolation (boot-sweep winner on vista) ---
  "v07-s175      0.02,1.75,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  -"
  # --- the 2026-08-01 B-drained lights, on vista, with warm haze ---------
  "v08-hz2L190   0.02,1.90,1.12,0.86    1.00,0.92,0.52,0.98,0.84,0.38  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  # --- ROUND 6 -----------------------------------------------------------
  # Round 5 cleared blue and sat and left warmth at 58-63 against 110 and DOF
  # at 1.8 against 3.0. These rows go after those two specifically.
  #
  # DOF fg:bg is a hi-freq std RATIO, so the honest way to move it on a camera
  # that (per docs/look-contract.md §5) will never carry `DepthOfField` again is
  # to soften the BACKGROUND with air rather than with a lens: thicker haze eats
  # high-frequency energy in the bg zone (10-40% height) and leaves the fg zone
  # (72-95%) alone. v10 doubles it, v13 stacks it on the warmest safe frame.
  "v10-thick     0.02,2.05,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62;VOXELFORGE_LOOK_HAZE=0.0140"
  # Warmth needs the band's mean RED to rise; saturation cannot supply it
  # because 60% of the vista band is green-dominant grass, where saturation
  # pushes R DOWN (measured: green-px mean R 65.1 -> 41.3 across sat 1.05 ->
  # 1.35). Light hue can, up to the magenta safety envelope of
  # docs/gate3-colour-review-2026-08-01.md §5.4: G-B >= 0.30 AND G >= 0.85 R.
  # [1.00, 0.85, 0.50] is that envelope's warmest corner — G-B 0.35, G/R 0.85.
  "v11-orange    0.02,1.75,1.12,0.86    1.00,0.85,0.50,1.00,0.85,0.50  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  # Temperature is a chromatic-adaptation MATRIX, not a per-channel gain, and
  # scripts/wb_matrix.py pins its magenta onset at ~0.099. 0.05 is the last
  # untested chromatic lever with real headroom under that onset.
  "v12-temp05    0.05,1.75,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  # Everything at once — the frontier row. Not a ship candidate, a ceiling probe:
  # if warmth still will not reach 110 here, it is not reachable from the grade.
  "v13-max       0.05,1.90,1.12,0.86    1.00,0.85,0.50,1.00,0.85,0.50  VOXELFORGE_LOOK_HAZECOL=0.94,0.66,0.26,0.62;VOXELFORGE_LOOK_HAZE=0.0140"
  # --- ROUND 7 -----------------------------------------------------------
  # v13 cleared warmth/blue/sat (151.5 / 8.2 / 95.5) but it is a ceiling probe,
  # not a look: it stacks 2x haze density on top of orange lights on top of a
  # temperature push, and 2x density means 86% opacity at 100 blocks against the
  # shipped 40% — a world seen through soup. v12 is the interesting row, because
  # temperature 0.05 ALONE (shipped haze density, shipped-ish lights) got warmth
  # 59.7 -> 97.0 and blue to 5.6. These rows find the cheapest row that still
  # clears, by backing the expensive knobs off one at a time.
  "v20-t05nohz   0.05,1.90,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.94,0.66,0.26,0.62"
  "v21-t05hz100  0.05,1.90,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.94,0.66,0.26,0.62;VOXELFORGE_LOOK_HAZE=0.0100"
  "v22-t07       0.07,1.75,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.90,0.68,0.34,0.62"
  "v23-t05s175   0.05,1.75,1.12,0.86    1.00,0.92,0.62,0.96,0.90,0.60  VOXELFORGE_LOOK_HAZECOL=0.94,0.66,0.26,0.62;VOXELFORGE_LOOK_HAZE=0.0100"
)

WANT=("$@")
run_row() {
  local label=$1 grade=$2 light=$3 extra=${4:--}
  local stem="$OUT/vista-${label}"
  local -a EX=()
  [ "$grade" != "-" ] && EX+=("VOXELFORGE_LOOK_GRADE=$grade")
  [ "$light" != "-" ] && EX+=("VOXELFORGE_LOOK_LIGHT=$light")
  # EXTRA carries 0..n `NAME=value` pairs separated by ';' — a row needs two
  # when it moves haze hue and haze density together.
  if [ "$extra" != "-" ]; then
    local IFS=';'
    for kv in $extra; do [ -n "$kv" ] && EX+=("$kv"); done
  fi
  echo "--- $label  GRADE=$grade  LIGHT=$light  EXTRA=$extra"
  env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_CAM=35,-18,26 \
      VOXELFORGE_LOOK_QUALITY=ultra "${EX[@]}" \
      VOXELFORGE_SHOT="$stem.png" "$BIN" >"$stem.log" 2>&1
  local rc=$?
  if [ ! -s "$stem.png" ]; then echo "  MISS (exit=$rc)"; tail -3 "$stem.log"; return 1; fi
  grep -qE 'panicked|B0001' "$stem.log" && echo "  !! PANIC"
  python scripts/_flamingo_dehud2.py "$stem.png" >/dev/null 2>&1
  python scripts/grade_axes.py "${stem}-nohud2.png" 2>&1 | tail -8
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
echo "=== vista sweep done ==="
