#!/usr/bin/env bash
# _kevin_vfx_deliver.sh — drain-wait -> build -> render -> sheet, one background job.
#
# Exists because the box currently has several teammates' builds running, and the
# LANES.md build-safety rule is absolute: never build in parallel (a stacked rustc
# is how the box runs out of process/commit headroom and dies 0xc0000142). So this
# job parks until cargo/rustc/link drain, THEN does its (cheap, 1-rustc) build.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="target-kevin"
LOG="_kevin_vfx_deliver.log"
: > "$LOG"

count_busy() {
  powershell -NoProfile -Command \
    "(Get-Process cargo,rustc,link -ErrorAction SilentlyContinue | Measure-Object).Count" \
    2>/dev/null | tr -d ' \r\n'
}

now() { date +%H:%M:%S; }

echo "[$(now)] waiting for cargo/rustc/link to drain (no parallel build)" | tee -a "$LOG"
WAITED=0
while true; do
  n="$(count_busy)"
  n="${n:-0}"
  if [ "$n" -le 0 ]; then
    echo "[$(now)] box drained after ${WAITED}s" | tee -a "$LOG"
    break
  fi
  WAITED=$((WAITED + 15))
  if [ "$WAITED" -ge 5400 ]; then
    echo "[$(now)] TIMEOUT ${WAITED}s: $n build process(es) still running — aborting rather than build in parallel" | tee -a "$LOG"
    exit 1
  fi
  echo "[$(now)] $n build process(es) running — waited ${WAITED}s" | tee -a "$LOG"
  sleep 15
done

echo "[$(now)] building voxelforge_shot -> $TARGET (warm deps, ~1 rustc + link)" | tee -a "$LOG"
if ! bash scripts/build_safe.sh build --bin voxelforge_shot --target-dir "$TARGET" >> "$LOG" 2>&1; then
  echo "[$(now)] BUILD FAILED — see $LOG" | tee -a "$LOG"
  exit 1
fi
echo "[$(now)] build ok" | tee -a "$LOG"

echo "[$(now)] rendering before/after pairs" | tee -a "$LOG"
if ! bash scripts/_kevin_vfx_pairs.sh "$TARGET" >> "$LOG" 2>&1; then
  echo "[$(now)] RENDER FAILED — see $LOG" | tee -a "$LOG"
  exit 1
fi

python scripts/_kevin_vfx_sheet.py >> "$LOG" 2>&1
echo "[$(now)] DONE" | tee -a "$LOG"
