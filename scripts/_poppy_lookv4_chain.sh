#!/usr/bin/env bash
# Poppy — build → gate → shoot → measure, in one command, for the lighting-v2/v3
# before/after plates.
#
# Reads `_poppy_lookv4_build.done` rather than polling the process table. That is
# a scar, not a style choice: a `tasklist` full of rustc.exe proves SOMEBODY on
# this box is building, and on a repo running five cargo lanes at once the odds
# that they are yours are not good. The marker file is written by the build
# script's own last line, so it is the build's own word.
#
# The gate between build and shoot is the other scar: one binary, two GEN values
# is only a valid A/B if the binary actually contains the v3 rig.
#
# Usage: bash scripts/_poppy_lookv4_chain.sh [outdir]
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT=$PWD
DONE=$ROOT/_poppy_lookv4_build.done
LOG=$ROOT/_poppy_lookv4_build.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OUT="${1:-$ROOT/docs/assets/look}"

echo "=== [1/4] waiting for $(basename "$DONE") ==="
waited=0
while [[ ! -f "$DONE" ]]; do
  sleep 15
  waited=$((waited + 15))
  # A build that is alive moves its own target dir. Cheap liveness that does not
  # depend on reading anyone else's process list.
  printf '\r  %4ds  log=%s lines  target touched=%s' \
    "$waited" "$(wc -l < "$LOG" 2>/dev/null || echo 0)" \
    "$(date -r "$ROOT/target-poppy" +%H:%M:%S 2>/dev/null || echo '-')"
done
echo
echo "  $(cat "$DONE")"

# `grep -c '^error'` is the whole verdict. `tail` shows cargo's last line, and
# cargo prints `Compiling` AFTER a failure, so the last line is not the answer.
errs=$(grep -c '^error' "$LOG" || true)
mine=$(grep -c '^error.*look\.rs' "$LOG" || true)
grep -A2 '^error' "$LOG" | grep -E '^\s+--> ' | sed 's/^/  /' | sort | uniq -c | sort -rn
echo "=== [2/4] errors: $errs total ==="
if [[ "$errs" -ne 0 ]]; then
  # look.rs is this lane. quest.rs is Sun's and is not mine to patch.
  echo "  errors naming look.rs: $mine"
  [[ "$mine" -eq 0 ]] && echo "  -> the look lane is clean; the build is held by another lane's file."
  echo "  no exe was produced — stopping before the gate."
  exit 1
fi

echo "=== [3/4] gate ==="
bash scripts/_poppy_lookv3_gate.sh "$EXE" || exit 1

echo "=== [4/4] shoot + measure -> $OUT ==="
mkdir -p "$OUT"
cmd //c "$(cygpath -w "$ROOT/scripts/_poppy_lookv3_shoot.cmd")" "$(cygpath -w "$EXE")" "$(cygpath -w "$OUT")"

shopt -s nullglob
shot=("$OUT"/*_before.png "$OUT"/*_after.png)
echo "  ${#shot[@]} frames on disk"
[[ ${#shot[@]} -eq 6 ]] || { echo "  EXPECTED 6 — capture is incomplete, not grading it."; exit 1; }

python scripts/_poppy_lookv3_pairs.py "$OUT"
