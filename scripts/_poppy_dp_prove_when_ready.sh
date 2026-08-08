#!/usr/bin/env bash
# Rebuild + prove the dodge/parry depth layer, queued politely behind whatever
# else the office is running.
#
# Three rules it follows, all learned the hard way:
#  1. Never start a build while another cargo is up — this machine has eaten
#     0xc0000142 / LNK1102 linker deaths from parallel builds (see
#     scripts/_poppy_build_watch.sh).
#  2. Never launch the game while another voxelforge.exe is up — two windowed
#     Bevy apps fighting over the GPU turns a proof run into a timeout that has
#     nothing to do with the mechanic being graded.
#  3. Never relax `stale_guard`. If a teammate's edit lands mid-build, rebuild —
#     do not grade a binary that predates the source.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

TARGET_DIR=target-int
BIN=./$TARGET_DIR/debug/voxelforge.exe
OUT=_dp_prove.log
BUILDLOG=_dp_rebuild.log
ATTEMPTS=3

busy() { tasklist 2>/dev/null | grep -qiE '^(cargo|rustc|voxelforge)\.exe'; }

wait_for_quiet() {
  local what="$1" i
  for i in $(seq 1 80); do            # 80 x 30s = 40 min ceiling
    busy || { echo "[dp] $(date '+%H:%M:%S') machine quiet — $what"; return 0; }
    [ $((i % 4)) -eq 0 ] && echo "[dp] $(date '+%H:%M:%S') still busy, waiting ($what)"
    sleep 30
  done
  echo "[dp] $(date '+%H:%M:%S') machine never went quiet — giving up"
  return 1
}

for attempt in $(seq 1 $ATTEMPTS); do
  echo "=== attempt $attempt/$ATTEMPTS ==="
  wait_for_quiet "starting incremental build" || { echo "DP_PROVE_STATUS=BLOCKED_MACHINE_BUSY"; exit 1; }

  echo "[dp] $(date '+%H:%M:%S') cargo build --bin voxelforge --target-dir $TARGET_DIR -j 2"
  CARGO_PROFILE_DEV_DEBUG=0 cargo build --bin voxelforge --target-dir "$TARGET_DIR" -j 2 \
    >"$BUILDLOG.$attempt" 2>&1
  rc=$?
  echo "[dp] $(date '+%H:%M:%S') build exit=$rc  $(grep -E '^(    Finished|error)' "$BUILDLOG.$attempt" | tail -1)"
  if [ "$rc" -ne 0 ]; then
    grep -E '^error' "$BUILDLOG.$attempt" | head -10
    echo "DP_PROVE_STATUS=BUILD_FAILED_exit_$rc"
    exit 1
  fi

  newer=$(find client/src -name '*.rs' -newer "$BIN" 2>/dev/null)
  if [ -n "$newer" ]; then
    echo "[dp] sources moved during the build:"; echo "$newer" | sed 's/^/    /'
    echo "[dp] rebuilding rather than grading a stale binary"
    continue
  fi

  wait_for_quiet "starting the proof run" || { echo "DP_PROVE_STATUS=BLOCKED_MACHINE_BUSY"; exit 1; }
  echo "[dp] $(date '+%H:%M:%S') scripts/prove_dodge_parry.sh against $BIN"
  BIN="$BIN" RUN_SECS=120 bash scripts/prove_dodge_parry.sh >"$OUT" 2>&1
  prc=$?
  echo "DP_PROVE_STATUS=exit_$prc"
  tail -40 "$OUT"
  exit $prc
done

echo "[dp] sources kept moving for $ATTEMPTS builds — a teammate is editing live"
echo "DP_PROVE_STATUS=BLOCKED_SOURCES_MOVING"
exit 1
