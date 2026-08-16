#!/usr/bin/env bash
# Poppy — re-run the look-v2 build once the box can survive it.
#
# The 16:33 run died EXIT=101 on two E0594s in `vfx.rs:1413` that are ALREADY
# fixed on disk (vfx.rs mtime 16:37 > the snapshot that compile read; the loop
# now reads `for (gtf, mut emitter) in &mut cams`). So there is nothing to
# repair here — the build simply has to read the file again.
#
# IT DOES NOT FIRE IMMEDIATELY, AND THAT IS THE POINT. Four cargo builds were
# live at 16:40. A fifth is how this lane previously earned a
# `STATUS_DLL_INIT_FAILED` out of `slotmap`'s build script: 0xc0000142 is the box
# running out of commit headroom, not a code fault, and `-j 1` does not help
# because the peak comes from the NUMBER OF CONCURRENT BUILDS, not the job count
# inside one. So this waits for the lane to thin to <= 2 other builds, then goes.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
TRACE="$ROOT/_poppy_lookv2_requeue.trace"
: > "$TRACE"

others() { # cargo.exe processes that are not this script's own child
  tasklist 2>/dev/null | grep -ci 'cargo\.exe' || true
}

waited=0
while [[ $(others) -gt 2 ]]; do
  sleep 20
  waited=$((waited + 20))
  echo "$(date '+%H:%M:%S') waiting, cargo.exe count=$(others), ${waited}s" >> "$TRACE"
  # Never wait forever: 25 min and it goes anyway, with the wait on the record.
  [[ $waited -ge 1500 ]] && { echo "TIMEOUT — building anyway" >> "$TRACE"; break; }
done
echo "$(date '+%H:%M:%S') lane clear enough (count=$(others)) after ${waited}s — building" >> "$TRACE"

cmd //c "$(cygpath -w "$ROOT/_poppy_lookv2_build.cmd")" >> "$TRACE" 2>&1

echo "$(date '+%H:%M:%S') done: $(cat "$ROOT/_poppy_lookv2_build.done" 2>/dev/null)" >> "$TRACE"
echo "errors: $(grep -c '^error' "$ROOT/_poppy_lookv2_build.log")" >> "$TRACE"
