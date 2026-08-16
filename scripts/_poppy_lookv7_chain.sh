#!/usr/bin/env bash
# Poppy -- v7 chain: wait for the lane build -> verdict by `grep -c '^error'`
# ONLY -> prove the relink -> run the ev100 ladder on all three plates.
#
# Detached and unattended on purpose. The last round of this task stalled with
# the build still queued behind another lane and NOTHING downstream armed, so
# when the queue cleared the ladder still had not run. Everything after the
# build is wired here instead of being waited on by hand.
#
# THE VERDICT IS `grep -c '^error'` ON THE FULL LOG. Not the tail (cargo prints
# `Compiling` lines after a failure), not the pipe's exit code (that is the last
# stage's), not lane-build's own exit (which is a stricter conjunction and is
# printed too, but the error count is what decides whether an exe is usable).
#
# Usage: bash scripts/_poppy_lookv7_chain.sh
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
DONE=$ROOT/_poppy_lookv7_build.done
LANE=$ROOT/_poppy_lookv7_lane.log
LOG=$ROOT/_poppy_build.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OLD_SHA=$(awk '{print $1}' "$ROOT/_poppy_lookv7_oldexe.sha256" 2>/dev/null || echo none)

echo "=== [1/4] waiting for the poppy lane build ==="
waited=0
while [[ ! -f "$DONE" ]]; do
  sleep 20
  waited=$((waited + 20))
  printf '  %5ds  lane=%s  cargo_log=%s lines\n' "$waited" \
    "$(tail -n1 "$LANE" 2>/dev/null | tr -d '\r')" \
    "$(wc -l < "$LOG" 2>/dev/null || echo 0)"
done
echo "  $(cat "$DONE")"
grep -E '^(LANE_BUSY_CHECK|BUILD_DONE|EXE_AFTER|GATE|VERDICT)' "$LANE" || true

echo
echo "=== [2/4] build verdict: grep -c '^error' on the full cargo log ==="
errs=$(grep -c '^error' "$LOG" 2>/dev/null || true)
mine=$(grep -c '^error.*look\.rs' "$LOG" 2>/dev/null || true)
warns=$(grep -c '^warning' "$LOG" 2>/dev/null || true)
echo "  errors=$errs  (naming look.rs: $mine)   raw '^warning' line count=$warns"
if [[ "${errs:-1}" -ne 0 ]]; then
  grep -B2 -A8 '^error' "$LOG" | head -80
  echo "  no usable exe -- stopping before the ladder."
  exit 1
fi
if ! grep -q '^ *Finished' "$LOG"; then
  echo "  errors=0 but no 'Finished' line -- the link never completed. Not shooting."
  exit 1
fi
echo "  $(grep -m1 '^ *Finished' "$LOG" | tr -d '\r')"

echo
echo "=== [3/4] relink proof ==="
new_sha=$(sha256sum "$EXE" | awk '{print $1}')
echo "  old $OLD_SHA"
echo "  new $new_sha"
if [[ "$new_sha" == "$OLD_SHA" ]]; then
  echo "  FAIL -- byte-identical to the pre-build exe. The link did not happen."
  exit 1
fi
echo "  PASS -- different binary from the one on disk before this build."
echo "  mtime $(date -r "$EXE" '+%F %T')"

echo
echo "=== [4/4] ev100 ladder, all three plates ==="
bash scripts/_poppy_lookv7_expo_ladder.sh "$EXE"
echo "LADDER EXIT=$?"
echo "CHAIN DONE"
