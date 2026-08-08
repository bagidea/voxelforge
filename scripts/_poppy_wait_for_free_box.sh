#!/usr/bin/env bash
# Wait until this machine has no cargo build running, then exit.
#
# The perf probe cannot share the box: rustc dies with 0xc0000142
# STATUS_DLL_INIT_FAILED when a second build is already holding the commit
# charge (killed twice: 20:39 and 20:50). So the probe queues instead of racing.
#
# Two consecutive clear checks, not one — cargo.exe can blink between the build
# and the link step, and a single clear sample would fire the all-clear early.
set -u
CLEAR=0
for _ in $(seq 1 240); do   # 240 x 20s = 80 min cap
  if tasklist 2>/dev/null | grep -qi 'cargo\.exe'; then
    CLEAR=0
  else
    CLEAR=$((CLEAR + 1))
    [ "$CLEAR" -ge 2 ] && { echo "BOX_FREE — no cargo.exe for 2 consecutive checks"; exit 0; }
  fi
  sleep 20
done
echo "TIMEOUT — a build was still running after 80 min"
exit 1
