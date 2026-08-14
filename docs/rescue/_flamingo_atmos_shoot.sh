#!/usr/bin/env bash
# _flamingo_atmos_shoot.sh — the before/after set for Bevy 0.19's built-in atmosphere.
#
# THREE ARMS, AND THE THIRD IS THE ONE THAT MAKES THIS HONEST:
#   before        — `_flamingo_before.exe`, a COPY of the pre-change release exe
#                   taken before `cargo build` could overwrite it. A commit sha
#                   does not date a binary (memory: commit time != build time),
#                   so the A side is a file pinned on disk, not a claim.
#   after-atmos   — the new exe, env UNSET. The shipped path, byte for byte.
#   after-noatmos — the SAME new exe with `VOXELFORGE_LOOK_ATMOS=off`. Same
#                   binary, same scene, only the sky swapped — which is what
#                   separates "the atmosphere did this" from "the rebuild did
#                   this". Every `_LOOK_*` hook in this lane exists for exactly
#                   this and this change does not get to skip it.
#
# TWO FRAMINGS, because the pinned one cannot see the thing under test:
#   vista   35,-18,26 — the framing EVERY gate row in this repo is shot on
#                       (`vista_grade_sweep.sh:107`, `verify_baked_grade.sh:56`).
#                       Measured here: 0 sky pixels. It is a wall-and-campfire
#                       close-up, so it can only prove the sky change did NOT
#                       regress the graded frame — it can never show the sky.
#   horizon 0,1,18    — pitch ~level, the pose Rose's probe used
#                       (`_rose_cap_skyprobe.sh`, `_rose_probe_camup2.png`).
#                       This one has sky in it, so this is where the sky is read.
#
# Usage: bash scripts/_flamingo_atmos_shoot.sh
set -uo pipefail
cd "$(dirname "$0")/.."

BEFORE="${BEFORE:-./_flamingo_before.exe}"
AFTER="${AFTER:-./target/release/voxelforge.exe}"
OUT="${OUT:-_flamingo_atmos}"
mkdir -p "$OUT"

stamp() { ls -l --time-style=+%Y-%m-%d\ %H:%M:%S "$1" | awk '{print $5" bytes  "$6" "$7}'; }
for b in "$BEFORE" "$AFTER"; do
  [ -f "$b" ] || { echo "no binary $b"; exit 2; }
  echo "BIN $b  $(stamp "$b")"
done
# The two exes MUST differ, or "after" is "before" under another name.
if [ "$(md5sum <"$BEFORE" | cut -d' ' -f1)" = "$(md5sum <"$AFTER" | cut -d' ' -f1)" ]; then
  echo "REFUSED  before and after are byte-identical — the build did not relink"; exit 2
fi

shoot() {                       # shoot <arm> <framing> <cam> [EXTRA=env ...]
  local arm=$1 framing=$2 cam=$3; shift 3
  local bin="$AFTER"; [ "$arm" = before ] && bin="$BEFORE"
  local png="$OUT/${framing}-${arm}.png"
  echo "--- $framing/$arm  cam=$cam  ${*:-(no extra env)}"
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_CAM="$cam" \
      VOXELFORGE_LOOK_QUALITY=ultra \
      "$@" \
      VOXELFORGE_SHOT="$png" \
      timeout 150 "$bin" >"$OUT/${framing}-${arm}.log" 2>&1
  local rc=$?
  if [ ! -s "$png" ]; then echo "   MISS (exit=$rc)"; tail -3 "$OUT/${framing}-${arm}.log"; return 1; fi
  echo "   exit=$rc  $(wc -c <"$png" | tr -d ' ') bytes"
  grep -aqE 'panicked|B0001' "$OUT/${framing}-${arm}.log" && echo "   !! PANIC"
  # The one line that proves the new code RAN, not merely compiled.
  grep -aE 'LOOK atmosphere spawned|LOOK sky-dome spawned' "$OUT/${framing}-${arm}.log" | sed 's/^/   /'
  python scripts/_flamingo_dehud2.py "$png" >/dev/null 2>&1
}

for f in "vista 35,-18,26" "horizon 0,1,18"; do
  set -- $f
  shoot before        "$1" "$2"
  shoot after-atmos   "$1" "$2"
  shoot after-noatmos "$1" "$2" VOXELFORGE_LOOK_ATMOS=off
done

echo
echo "=== files ==="
ls -l --time-style=+%H:%M:%S "$OUT"/*.png | awk '{print $5"  "$6"  "$7}'
