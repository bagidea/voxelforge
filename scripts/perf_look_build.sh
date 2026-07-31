#!/usr/bin/env bash
# Build the look perf probe into an isolated target dir.
#
# TWO ATTEMPTS DIED BEFORE THIS SCRIPT LOOKED LIKE THIS:
#   20:39  `could not compile windows` / `bevy_reflect` — rustc killed mid-crate,
#          no diagnostic at all (the OOM signature)
#   20:50  `slotmap` build-script, exit code 0xc0000142 STATUS_DLL_INIT_FAILED
#
# Neither was a source bug. Both times a release build in another lane was
# already resident and the box was out of commit headroom. The 20:50 run was
# `-j 1` — so LOWERING THE JOB COUNT DOES NOT FIX THIS. The peak comes from two
# cargo builds coexisting, not from one build's parallelism. The fix is
# exclusivity, which is why the guard below refuses to start rather than
# quietly producing a third dead log.
set -u
LOG="${1:-_poppy_perf_build.log}"

# Refuse to be the second build on the machine. `Get-CimInstance`, not
# `tasklist` — tasklist tells you a cargo.exe exists, not whose it is, and
# reading someone else's rustc as your own is exactly how the last run got
# reported as "still compiling" 10 minutes after it had died.
others=$(powershell -NoProfile -Command \
  "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\").Count" 2>/dev/null | tr -d '[:space:]')
if [ "${others:-0}" != "0" ]; then
  echo "REFUSING TO BUILD — $others cargo.exe already running (another lane)." >&2
  echo "Wait for it to finish; a concurrent build is what killed the 20:39 and" >&2
  echo "20:50 attempts. Check with:" >&2
  echo "  Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\" | Select CommandLine" >&2
  exit 3
fi

: > "$LOG"
cargo build -p voxelforge --bin voxelforge_perf --profile perf \
  --target-dir target-poppy -j 2 >>"$LOG" 2>&1
echo "BUILD_EXIT=$?" >>"$LOG"

# Verdict on the log, not on a pipe's exit code — `cargo ... | tail` reports
# tail's status, which is always 0.
grep '^error' "$LOG" || echo "no 'error' lines in $LOG"
grep 'BUILD_EXIT' "$LOG"
