#!/usr/bin/env bash
# Flamingo — wait for the box, rebuild, then shoot the before/after pair (2026-08-05).
#
# WHY IT WAITS. Two cargo/rustc pairs were already live when this was written.
# A third concurrent release build on this machine ends in STATUS_DLL_INIT_FAILED
# — the linker runs out of the desktop heap and the exe that lands is a corpse
# that dies at startup, which then reads as "the look change broke the game".
# So: poll until no cargo/rustc is running, THEN build.
#
# WHY IT SHOOTS BOTH SIDES FROM THE NEW BINARY. The "before" is the OLD grade
# constants replayed through VOXELFORGE_LOOK_GRADE / VOXELFORGE_LOOK_LIGHT on the
# SAME exe, and the "after" is the same exe with those env vars UNSET. One
# binary, one scene, one variable — and the unset run is what proves the baked
# default actually reproduces the tuned frame instead of needing an env crutch.
set -uo pipefail
cd "$(dirname "$0")/.."
OUT=_fl_grade_final
mkdir -p "$OUT"
LOG="$OUT/chain.log"
: > "$LOG"
say() { echo "$(date '+%H:%M:%S') $*" | tee -a "$LOG"; }

# ---- 1. wait for a free box -------------------------------------------------
say "waiting for cargo/rustc to clear..."
for i in $(seq 1 240); do   # up to 40 min
  n=$(tasklist 2>/dev/null | grep -ciE '^(cargo|rustc)\.exe' || true)
  [ "${n:-0}" -eq 0 ] && { say "box free after ${i} polls"; break; }
  sleep 10
done
n=$(tasklist 2>/dev/null | grep -ciE '^(cargo|rustc)\.exe' || true)
if [ "${n:-0}" -ne 0 ]; then say "STILL BUSY ($n procs) — refusing to stack a build"; exit 3; fi

# ---- 2. build ---------------------------------------------------------------
say "building release (CARGO_PROFILE_RELEASE_DEBUG=0)..."
CARGO_PROFILE_RELEASE_DEBUG=0 cargo build --release -p voxelforge > "$OUT/build.log" 2>&1
BUILD_RC=$?
say "build exit=$BUILD_RC"
ERRS=$(grep -c '^error' "$OUT/build.log" || true)
say "grep '^error' count = $ERRS"
if [ "$BUILD_RC" -ne 0 ] || [ "${ERRS:-0}" -ne 0 ]; then
  say "BUILD FAILED — first errors:"
  grep -n '^error' "$OUT/build.log" | head -20 | tee -a "$LOG"
  exit 1
fi
ls -la target/release/voxelforge.exe | tee -a "$LOG"

# ---- 3. shoot before/after on two framings ----------------------------------
# OLD = the constants that were on disk before this change.
OLD_GRADE="0.02,1.05,1.12,0.86"
OLD_LIGHT="1.00,0.84,0.62,0.96,0.84,0.66"

shoot() { # $1 stem  $2 framing  $3 side(before|after)
  local stem=$1 framing=$2 side=$3
  local -a MODE=(VOXELFORGE_PLAY=1)
  [ "$framing" = vista ] && MODE+=(VOXELFORGE_LOOK_CAM=35,-18,26)
  local -a EX=()
  [ "$side" = before ] && EX=(VOXELFORGE_LOOK_GRADE="$OLD_GRADE" VOXELFORGE_LOOK_LIGHT="$OLD_LIGHT")
  say "shoot $stem ($framing/$side)"
  env "${MODE[@]}" "${EX[@]}" VOXELFORGE_LOOK_QUALITY=high \
      VOXELFORGE_SHOT="$OUT/$stem.png" ./target/release/voxelforge.exe \
      > "$OUT/$stem.log" 2>&1
  local rc=$?
  { echo "framing=$framing side=$side exit=$rc"
    echo "env: ${MODE[*]} ${EX[*]-<none - baked default>}"
    echo "exe_mtime: $(date -r target/release/voxelforge.exe '+%Y-%m-%dT%H:%M:%S')"
    echo "exe_sha256: $(sha256sum target/release/voxelforge.exe | awk '{print $1}')"
  } > "$OUT/$stem.runlog"
  if [ ! -s "$OUT/$stem.png" ]; then say "  MISS exit=$rc"; tail -3 "$OUT/$stem.log" | tee -a "$LOG"; return 1; fi
  grep -qE 'panicked|B0001' "$OUT/$stem.log" && say "  !! PANIC in log"
  python scripts/_flamingo_dehud2.py "$OUT/$stem.png" >/dev/null 2>&1
  say "  ok exit=$rc"
}

for framing in boot vista; do
  for side in before after; do
    shoot "${framing}-${side}" "$framing" "$side"
  done
done
# repeat the after side once, so the report can quote a shot-to-shot error bar
shoot "boot-after-b" boot after
shoot "vista-after-b" vista after

say "=== chain done ==="
