#!/usr/bin/env bash
# Detached runner for the R-key runtime proof.
#
# WHY A WRAPPER. `_poppy_rkey_proof.sh` already queues behind a foreign build
# (STEP 1), but it cannot see *my own* `_poppy_rkey_check.sh`, which is queued
# for the same all-clear. Two scripts watching one signal fire together, and two
# cargos on this box is the 0xc0000142 STATUS_DLL_INIT_FAILED trap
# (`docs/LANES.md`, "Build safety"). So the chain waits on a FILE signal —
# `CHECK_EXIT=` landing in `_poppy_rkey_check.log` — not on a process list:
# `tasklist`/`ps -ef` show `bash.exe`, never which script it is running.
#
# TARGET DIR. `docs/LANES.md` §"The build lock" says target-int, or
# target-combat when target-int is unusable. There is no `target-int` on this
# disk at all — creating one means a cold build, i.e. maximum rustc spawns,
# i.e. maximum STATUS_DLL_INIT_FAILED risk. `target-combat` is this lane's own
# dir and is warm (it carries the 17:15 perf binary), so it is the fallback the
# rule names.
#
# Run it detached and walk away — never hold it in the foreground:
#   Start-Process bash -ArgumentList 'scripts/_poppy_rkey_chain.sh'
set -uo pipefail
cd "$(dirname "$0")/.."

echo "=== CHAIN start @ $(date +%H:%M:%S) (pid $$) ==="

# 1. Let the lane's own `cargo check` finish first (one cargo at a time).
if [ -f _poppy_rkey_check.log ]; then
  for _ in $(seq 1 180); do            # 180 x 20s = 60 min cap
    if grep -q 'CHECK_EXIT=' _poppy_rkey_check.log; then
      echo "CHECK done: $(grep 'CHECK_EXIT=' _poppy_rkey_check.log)"
      break
    fi
    sleep 20
  done
  grep -q 'CHECK_EXIT=' _poppy_rkey_check.log \
    || echo "CHAIN note: check never reported in 60 min — going ahead, STEP 1 still gates on the box"
fi

# 2. The proof itself (it does its own box-free wait, build, and real run).
bash scripts/_poppy_rkey_proof.sh target-combat
echo "=== CHAIN end @ $(date +%H:%M:%S) rc=$? ==="
