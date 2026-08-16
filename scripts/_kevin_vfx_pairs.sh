#!/usr/bin/env bash
# _kevin_vfx_pairs.sh — before/after pairs for kevin's VFX pass (blood + smoke +
# ambient dust). Same honest mute-pair methodology as render_vfx_pairs.sh — a
# "before" plate is the SAME beat with the emitters muted, not a different branch
# — extended with the DISSOLVE beat (blood burst + lingering pool, the CEO's
# "blood must read clearly on target" pass).
#
# Usage:  bash scripts/_kevin_vfx_pairs.sh [target-dir]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="${1:-target-kevin}"
EXE="$TARGET/debug/voxelforge_shot.exe"
[ -x "$EXE" ] || EXE="$TARGET/debug/voxelforge_shot"

if [ ! -x "$EXE" ]; then
  echo "FAIL no shot binary at $EXE — build first:"
  echo "  bash scripts/build_safe.sh build --bin voxelforge_shot --target-dir $TARGET"
  exit 1
fi

OUT="docs/assets/pairs"
mkdir -p "$OUT"

# Same framing constants as render_vfx_pairs.sh — keep in sync or the pairs stop
# being comparable with the shipped singles.
CAM_EYE="${VOXELFORGE_VFX_EYE:--7.90,4.50,0.30}"
CAM_AIM="${VOXELFORGE_VFX_AIM:--0.35,1.60,-0.55}"

shoot() {
  local beat="$1" mute="$2" png="$OUT/$3"
  local runlog="${png%.png}.runlog"
  echo "--- $beat mute=$mute -> $png"
  VOXELFORGE_VFX="$beat" VOXELFORGE_VFX_MUTE="$mute" \
    VOXELFORGE_VFX_EYE="$CAM_EYE" VOXELFORGE_VFX_AIM="$CAM_AIM" \
    VOXELFORGE_SHOT="$png" "$EXE" >"$runlog" 2>&1
  local rc=$?
  # Echo the frame the grab happened on: if the fixed 1/60 step ever stops working,
  # two plates of a pair land on different frames and it shows here.
  grep -h 'SHOT saved' "$runlog" 2>/dev/null | tail -1
  if [ "$rc" -ne 0 ]; then
    echo "FAIL $beat/$mute: renderer exited $rc"
    return 1
  fi
  # A green exit code with no PNG is the gate-that-cannot-fail trap — assert the
  # artefact, not the exit code.
  if [ ! -s "$png" ]; then
    echo "FAIL $beat/$mute: no PNG written at $png"
    return 1
  fi
  echo "OK   $beat/$mute: $(ls -l "$png" | awk '{print $5}') bytes"
}

pair() {
  shoot "$1" 1 "$2-a-before.png" || return 1
  shoot "$1" 0 "$2-b-after.png"  || return 1
}

rc=0
pair impact   impact   || rc=1
pair dissolve dissolve || rc=1
pair parry    parry    || rc=1
pair stagger  stagger  || rc=1
shoot impact 1 "impact-a-before-control.png" || rc=1

if [ "$rc" -eq 0 ]; then
  echo "KEVIN_VFX_PAIRS PASS 9/9 (4 pairs + control) -> $OUT"
else
  echo "KEVIN_VFX_PAIRS FAIL"
fi
exit "$rc"
