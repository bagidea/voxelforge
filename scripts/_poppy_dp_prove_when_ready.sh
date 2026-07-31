#!/usr/bin/env bash
# Wait for the in-flight integration build + its three gates to clear, then run
# the dodge/parry proof against that same fresh binary.
#
# Why it waits for the WHOLE integration pass rather than just the build: the
# gates in _yama_integration_pass.sh launch the game too, and two copies of a
# windowed Bevy app fighting over the GPU is how a proof run picks up a timeout
# that has nothing to do with the mechanic it is grading.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

PASSLOG=_yama_integration_pass.log
BIN=./target-int/debug/voxelforge.exe
OUT=_dp_prove.log

echo "[dp] $(date '+%H:%M:%S') waiting for the integration pass to finish"
for _ in $(seq 1 120); do            # 120 x 30s = 60 min ceiling
  grep -q 'PIPELINE_DONE' "$PASSLOG" 2>/dev/null && break
  sleep 30
done

if ! grep -q 'PIPELINE_DONE' "$PASSLOG" 2>/dev/null; then
  echo "[dp] $(date '+%H:%M:%S') integration pass never reached PIPELINE_DONE — giving up"
  echo "DP_PROVE_STATUS=BLOCKED_NO_BINARY" | tee -a "$OUT"
  exit 1
fi
echo "[dp] $(date '+%H:%M:%S') $(grep 'PIPELINE_DONE' "$PASSLOG" | tail -1)"

if [ ! -f "$BIN" ]; then
  echo "[dp] no binary at $BIN"
  echo "DP_PROVE_STATUS=BLOCKED_NO_BINARY" | tee -a "$OUT"
  exit 1
fi

echo "[dp] $(date '+%H:%M:%S') running scripts/prove_dodge_parry.sh against $BIN"
BIN="$BIN" RUN_SECS=120 bash scripts/prove_dodge_parry.sh >"$OUT" 2>&1
rc=$?
echo "DP_PROVE_STATUS=exit_$rc" | tee -a "$OUT"
tail -30 "$OUT"
exit $rc
