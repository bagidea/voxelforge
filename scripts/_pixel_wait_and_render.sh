#!/usr/bin/env bash
# _pixel_wait_and_render.sh — wait for the shot binary, then render the VFX pairs.
#
# Scratch driver for the pixel lane, not a gate. It exists because the build is
# long and this session has already been torn down twice mid-wait: chaining the
# render to the binary's arrival means a teardown costs a wait, not a rebuild.
#
# Two things this got wrong before, both of which cost a whole run:
#
#   1. The target dir and the build log were hardcoded to one lane's scratch
#      (target-vfx / _vfx_build.log). When the office serialised the builds into
#      plain `target/` with a different log, this script sat and timed out against
#      a directory that no longer existed. They are arguments now.
#
#   2. "The exe stopped growing" is NOT "the exe is new". A stale binary from an
#      earlier build is perfectly size-stable, so the old check would happily
#      hand a pre-fix binary to the renderer and the plates would silently show
#      the OLD behaviour. The freshness test is therefore mtime against the
#      SOURCE we are waiting on — size-stability only guards the link step on top
#      of that.
#
# Usage: bash scripts/_pixel_wait_and_render.sh [target-dir] [build-log] [source-file]
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${1:-target}"
LOG="${2:-_shino_serial.log}"
SRC_FILE="${3:-client/src/vfx.rs}"
EXE="$TARGET/debug/voxelforge_shot.exe"
[ -e "$EXE" ] || EXE="$TARGET/debug/voxelforge_shot"

if [ ! -f "$SRC_FILE" ]; then
  echo "FAIL no source file at $SRC_FILE — nothing to measure freshness against"
  exit 1
fi
SRC_MTIME=$(stat -c %Y "$SRC_FILE")
echo "waiting for $EXE newer than $SRC_FILE (mtime $SRC_MTIME), log=$LOG"

ready=0
for i in $(seq 1 240); do
  if grep -q '^error' "$LOG" 2>/dev/null; then
    echo "BUILD_FAILED"
    grep -m5 '^error' "$LOG"
    exit 1
  fi
  if [ -s "$EXE" ]; then
    m=$(stat -c %Y "$EXE" 2>/dev/null || echo 0)
    if [ "$m" -gt "$SRC_MTIME" ]; then
      # Fresh. Now require it to stop growing before running it — a half-linked
      # exe on Windows is readable and fails in a way that looks like a render bug.
      a=$(stat -c %s "$EXE" 2>/dev/null || echo 0)
      sleep 6
      b=$(stat -c %s "$EXE" 2>/dev/null || echo 0)
      if [ "$a" = "$b" ] && [ "$a" != "0" ]; then
        echo "EXE_READY bytes=$b mtime=$m (src $SRC_MTIME)"
        ready=1
        break
      fi
    fi
  fi
  sleep 15
done

if [ "$ready" -ne 1 ]; then
  echo "TIMEOUT no exe newer than $SRC_FILE at $EXE"
  exit 1
fi

echo "=== rendering pairs ==="
bash scripts/render_vfx_pairs.sh "$TARGET"
rc=$?
echo "RENDER_RC=$rc"

echo "=== sheet ==="
python scripts/make_vfx_pair_sheet.py
echo "SHEET_RC=$?"
exit "$rc"
