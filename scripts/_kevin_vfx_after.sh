#!/usr/bin/env bash
# _kevin_vfx_after.sh — drain-wait -> build voxelforge_vfx_proof -> render the
# "after" (NEW vfx.rs) plates. One background job; parks until cargo/rustc/link
# drain so it never builds in parallel (LANES.md build-safety: a stacked rustc is
# how the box dies 0xc0000142), then does its (cheap, warm-deps) build and renders.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="target-kevin"
LOG="_kevin_vfx_after.log"
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
  WAITED=$((WAITED + 20))
  if [ "$WAITED" -ge 5400 ]; then
    echo "[$(now)] TIMEOUT ${WAITED}s: $n build process(es) still running — aborting rather than build in parallel" | tee -a "$LOG"
    exit 1
  fi
  echo "[$(now)] $n build process(es) running — waited ${WAITED}s" | tee -a "$LOG"
  sleep 20
done

echo "[$(now)] building voxelforge_vfx_proof -> $TARGET (warm deps)" | tee -a "$LOG"
if ! bash scripts/build_safe.sh build --bin voxelforge_vfx_proof --target-dir "$TARGET" >> "$LOG" 2>&1; then
  echo "[$(now)] BUILD FAILED — see $LOG" | tee -a "$LOG"
  exit 1
fi
echo "[$(now)] build ok" | tee -a "$LOG"

EXE="$TARGET/debug/voxelforge_vfx_proof.exe"
[ -x "$EXE" ] || EXE="$TARGET/debug/voxelforge_vfx_proof"
OUT="docs/assets/vfx-kevin"
mkdir -p "$OUT"

shoot() {
  local beat="$1" mute="$2" png="$OUT/$3"
  echo "[$(now)] --- $beat mute=$mute -> $png" | tee -a "$LOG"
  VOXELFORGE_VFX="$beat" VOXELFORGE_VFX_MUTE="$mute" VOXELFORGE_SHOT="$png" \
    "$EXE" >"${png%.png}.runlog" 2>&1
  local rc=$?
  grep -h 'SHOT saved' "${png%.png}.runlog" 2>/dev/null | tail -1 | tee -a "$LOG"
  if [ "$rc" -ne 0 ]; then echo "[$(now)] FAIL $beat/$mute rc=$rc" | tee -a "$LOG"; return 1; fi
  [ -s "$png" ] || { echo "[$(now)] FAIL no PNG $png" | tee -a "$LOG"; return 1; }
  echo "[$(now)] OK $png $(ls -l "$png" | awk '{print $5}') bytes" | tee -a "$LOG"
}

rc=0
shoot impact 0 impact-after.png        || rc=1
shoot trail  0 trail-after.png         || rc=1
shoot dissolve 0 dissolve-after.png    || rc=1
shoot impact 1 impact-after-muted.png  || rc=1
shoot trail  1 trail-after-muted.png   || rc=1

if [ "$rc" -eq 0 ]; then
  echo "[$(now)] KEVIN_VFX_AFTER PASS (5 renders) -> $OUT" | tee -a "$LOG"
else
  echo "[$(now)] KEVIN_VFX_AFTER FAIL" | tee -a "$LOG"
fi
exit "$rc"
