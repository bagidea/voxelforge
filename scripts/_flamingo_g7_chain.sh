#!/usr/bin/env bash
# Flamingo G7 — wait for the sweep binary to finish linking, then fire the sweep.
#
# Detached on purpose. The session's idle watchdog cuts at 5 minutes and a build
# here runs longer than that, so holding the build in the foreground is how five
# agents got killed mid-compile in one night (docs/LANES.md). This script owns
# the wait instead, and the session only ever polls a log.
#
# BUILD VERDICT IS `grep '^error'` PLUS THE `Finished` LINE, NOTHING ELSE — not
# `tail`, not an exit code that came through a pipe. A warning block at the end
# of a green build reads exactly like a failure in a tail, and `$?` after a pipe
# is the pipe's.
set -u
cd "$(dirname "$0")/.."

LOG=_flamingo_g7/build.log
CHAIN=_flamingo_g7/chain.log
: > "$CHAIN"

for i in $(seq 1 300); do
  if grep -q '^error' "$LOG" 2>/dev/null; then
    echo "[$(date +%T)] BUILD FAILED — $(grep -c '^error' "$LOG") error line(s)" | tee -a "$CHAIN"
    grep '^error' "$LOG" | head -20 >> "$CHAIN"
    exit 1
  fi
  if grep -q '^ *Finished' "$LOG" 2>/dev/null; then
    echo "[$(date +%T)] build green: $(grep -m1 '^ *Finished' "$LOG")" | tee -a "$CHAIN"
    break
  fi
  echo "[$(date +%T)] building… $(wc -l < "$LOG") log lines" >> "$CHAIN"
  sleep 20
done

EXE=target-flamingo/perf/voxelforge.exe
if [ ! -f "$EXE" ]; then
  echo "[$(date +%T)] no exe at $EXE — stopping" | tee -a "$CHAIN"
  exit 1
fi
echo "[$(date +%T)] exe $(stat -c '%y %s' "$EXE")" | tee -a "$CHAIN"

echo "[$(date +%T)] sweep start" >> "$CHAIN"
powershell.exe -NoProfile -ExecutionPolicy Bypass \
  -File scripts/_flamingo_g7_sweep.ps1 _flamingo_g7/plan.txt _flamingo_g7 >> "$CHAIN" 2>&1
echo "[$(date +%T)] sweep done exit=$?" >> "$CHAIN"
