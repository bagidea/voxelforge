#!/usr/bin/env bash
# Poppy — wait for the queued look-v2 build to finish, then shoot the plates.
#
# The wait is on the `.done` stamp being NEWER than the failed 16:38:52 run, not
# on the file merely existing: the stale one is still on disk, and "the file is
# there" would fire the capture against a binary that never relinked. The build
# itself is `_poppy_lookv2_requeue.sh`, already queued behind the other lanes.
#
# Verdict is `grep -c '^error'`, never a tail and never a pipe's exit code —
# cargo keeps printing `Compiling` lines after it has already failed, and `tee`
# swallows its status on this box. Both of those have burned this lane before.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
DONE="$ROOT/_poppy_lookv2_build.done"
LOG="$ROOT/_poppy_lookv2_build.log"
EXE="$ROOT/target-poppy/perf/voxelforge.exe"
TRACE="$ROOT/_poppy_lookv2_chain.trace"

BEFORE=$(stat -c %Y "$DONE" 2>/dev/null || echo 0)
echo "$(date '+%H:%M:%S') waiting for a .done newer than $BEFORE" > "$TRACE"

waited=0
while [[ "$(stat -c %Y "$DONE" 2>/dev/null || echo 0)" == "$BEFORE" ]]; do
  sleep 20
  waited=$((waited + 20))
  [[ $((waited % 120)) == 0 ]] && echo "$(date '+%H:%M:%S') still waiting, ${waited}s" >> "$TRACE"
  [[ $waited -ge 5400 ]] && { echo "GAVE UP after 90 min" >> "$TRACE"; exit 3; }
done

ERRS=$(grep -c '^error' "$LOG")
{
  echo "$(date '+%H:%M:%S') build finished after ${waited}s"
  echo "  $(cat "$DONE")"
  echo "  errors: $ERRS"
  echo "  exe:    $(ls -la --time-style='+%Y-%m-%d %H:%M:%S' "$EXE" 2>&1)"
} >> "$TRACE"

if [[ "$ERRS" != "0" ]]; then
  echo "STOP — $ERRS error lines, not shooting a stale binary" >> "$TRACE"
  grep '^error' "$LOG" >> "$TRACE"
  exit 1
fi

echo "$(date '+%H:%M:%S') shooting plates" >> "$TRACE"
bash scripts/_poppy_lookv2_ab.sh >> "$TRACE" 2>&1
echo "$(date '+%H:%M:%S') capture done" >> "$TRACE"
