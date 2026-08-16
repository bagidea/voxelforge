#!/usr/bin/env bash
# _kevin_vfx_before.sh — render the "before" (OLD vfx.rs @ b420af7, pre-overhaul)
# plates for the honest old-code-vs-new-code comparison. Swaps vfx.rs in place,
# builds into the SAME warm target-kevin (only the voxelforge crate + link re-run,
# so spawn count stays low), renders, and restores vfx.rs from its own snapshot.
#
# MUST be run AFTER `_kevin_vfx_after.sh` has finished (serial build — never two
# cargo builds at once on this box, see LANES.md "Build safety").
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="target-kevin"
LOG="_kevin_vfx_before.log"
SNAP="_kevin_vfx_new.rs.snapshot"
OLD_REF="b420af7"   # parent of e9b5cd4 (the pre-overhaul vfx.rs)
: > "$LOG"

count_busy() {
  powershell -NoProfile -Command \
    "(Get-Process cargo,rustc,link -ErrorAction SilentlyContinue | Measure-Object).Count" \
    2>/dev/null | tr -d ' \r\n'
}
now() { date +%H:%M:%S; }

# ---- snapshot the current (new) vfx.rs, then drop in the old one ----------
cp client/src/vfx.rs "$SNAP"
git show "$OLD_REF:client/src/vfx.rs" > client/src/vfx.rs
echo "[$(now)] swapped vfx.rs -> $OLD_REF (snapshot at $SNAP)" | tee -a "$LOG"

# ---- drain-wait (no parallel build) ---------------------------------------
echo "[$(now)] waiting for cargo/rustc/link to drain" | tee -a "$LOG"
WAITED=0
while true; do
  n="$(count_busy)"; n="${n:-0}"
  if [ "$n" -le 0 ]; then echo "[$(now)] drained after ${WAITED}s" | tee -a "$LOG"; break; fi
  WAITED=$((WAITED + 20))
  if [ "$WAITED" -ge 5400 ]; then echo "[$(now)] TIMEOUT — aborting" | tee -a "$LOG"; exit 1; fi
  echo "[$(now)] $n running — waited ${WAITED}s" | tee -a "$LOG"
  sleep 20
done

# ---- build old vfx.rs (warm target — recompiles one crate) -----------------
echo "[$(now)] building old vfx.rs -> $TARGET" | tee -a "$LOG"
if ! bash scripts/build_safe.sh build --bin voxelforge_vfx_proof --target-dir "$TARGET" >> "$LOG" 2>&1; then
  echo "[$(now)] BUILD FAILED — restoring vfx.rs" | tee -a "$LOG"
  cp "$SNAP" client/src/vfx.rs && rm -f "$SNAP"
  exit 1
fi
echo "[$(now)] build ok" | tee -a "$LOG"

# ---- render before plates ---------------------------------------------------
EXE="$TARGET/debug/voxelforge_vfx_proof.exe"
[ -x "$EXE" ] || EXE="$TARGET/debug/voxelforge_vfx_proof"
OUT="docs/assets/vfx-kevin"
mkdir -p "$OUT"

shoot() {
  local beat="$1" png="$OUT/$2"
  echo "[$(now)] --- $beat -> $png" | tee -a "$LOG"
  VOXELFORGE_VFX="$beat" VOXELFORGE_VFX_MUTE=0 VOXELFORGE_SHOT="$png" \
    "$EXE" >"${png%.png}.runlog" 2>&1
  local rc=$?
  grep -h 'SHOT saved' "${png%.png}.runlog" 2>/dev/null | tail -1 | tee -a "$LOG"
  if [ "$rc" -ne 0 ]; then echo "[$(now)] FAIL $beat rc=$rc" | tee -a "$LOG"; return 1; fi
  [ -s "$png" ] || { echo "[$(now)] FAIL no PNG $png" | tee -a "$LOG"; return 1; }
  echo "[$(now)] OK $png $(ls -l "$png" | awk '{print $5}') bytes" | tee -a "$LOG"
}

# "before impact" = old impact beat; "before trail" = old parry beat (blade mid-
# swing, ribbon + ring both broken in the old code → a clean no-VFX swing frame).
rc=0
shoot impact impact-before.png  || rc=1
shoot parry  trail-before.png   || rc=1

# ---- restore the new vfx.rs and rebuild so the tree/artifact are consistent --
cp "$SNAP" client/src/vfx.rs && rm -f "$SNAP"
echo "[$(now)] restored new vfx.rs" | tee -a "$LOG"
if [ "$rc" -eq 0 ]; then
  echo "[$(now)] KEVIN_VFX_BEFORE PASS -> $OUT" | tee -a "$LOG"
else
  echo "[$(now)] KEVIN_VFX_BEFORE FAIL" | tee -a "$LOG"
fi
exit "$rc"
