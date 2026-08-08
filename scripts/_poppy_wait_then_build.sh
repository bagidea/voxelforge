#!/usr/bin/env bash
# Wait for the machine's cargo lane to go quiet, THEN build my two bins.
#
# Why a waiter instead of just building: `STATUS_DLL_INIT_FAILED` (0xc0000142)
# is what a second concurrent cargo gets on this box, and it has killed three
# builds in this repo already (look-perf-methodology.md §0). Lowering -j does
# not help; the peak comes from two builds coexisting. So the guard is
# exclusivity, and this script buys it by waiting instead of by refusing.
#
# It prints a line every poll on purpose. A silent 40-minute sleep is how a
# watchdog decides the session is idle and kills it, orphaning the build.
#
# Both bins in ONE cargo invocation: they share the whole dependency graph, so
# two invocations would be two full link passes for no reason -- and the second
# one would be exactly the concurrent build this script exists to avoid.
set -u
cd "$(dirname "$0")/.."

LOG="${1:-_poppy_cine/build_final.log}"
QUIET_NEEDED=2       # consecutive clear polls before we trust it
POLL=30
MAX_WAIT=5400        # 90 min, then give up rather than hang forever

: > "$LOG"
say() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

cargo_count() {
  powershell -NoProfile -Command \
    "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\").Count" 2>/dev/null \
    | tr -d '[:space:]'
}

clear_streak=0
waited=0
while [ "$waited" -lt "$MAX_WAIT" ]; do
  n=$(cargo_count); n=${n:-0}
  if [ "$n" = "0" ]; then
    clear_streak=$((clear_streak + 1))
    say "lane clear ($clear_streak/$QUIET_NEEDED)"
    [ "$clear_streak" -ge "$QUIET_NEEDED" ] && break
  else
    [ "$clear_streak" != "0" ] && say "lane re-taken, streak reset"
    clear_streak=0
    say "waiting — $n cargo.exe running (${waited}s elapsed)"
  fi
  sleep "$POLL"
  waited=$((waited + POLL))
done

if [ "$clear_streak" -lt "$QUIET_NEEDED" ]; then
  say "GAVE UP after ${waited}s — lane never went quiet"
  echo "BUILD_EXIT=99" >>"$LOG"
  exit 99
fi

say "BUILDING voxelforge + voxelforge_perf (release, -j 2) into target/"
# CARGO_PROFILE_RELEASE_DEBUG=0: dropping /DEBUG off the link line is what bought
# the headroom the last time a full release link died at 0xc0000142.
CARGO_PROFILE_RELEASE_DEBUG=0 cargo build --release -j 2 \
  --bin voxelforge --bin voxelforge_perf >>"$LOG" 2>&1
code=$?
echo "BUILD_EXIT=$code" >>"$LOG"
say "done, exit=$code"
grep -E '^error' "$LOG" | head -20
