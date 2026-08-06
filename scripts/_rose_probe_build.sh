#!/usr/bin/env bash
# Detached builder for the look perf probe (Rose).
#
# Launched from the Windows Task Scheduler (schtasks), NOT the agent shell, so
# it survives the agent session ending (the Schedule service owns it, not the
# Claude process tree). The probe build has died repeatedly as an orphaned
# background task; this is the fix.
#
# Waits for a free cargo slot — two concurrent cargo builds on this box die with
# STATUS_DLL_INIT_FAILED (methodology look-perf-methodology.md §0) — then builds
# the probe alone into target-rose, -j 8 (faster than the -j 2 contention guard,
# safe when solo). Retries up to 5 times on contention death; stops cold on a
# real compile error (so Rose can read it). Logs everything to _rose_perf_build.log.
set -u
cd "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge" || exit 13
LOG="_rose_perf_build.log"
EXE="target-rose/perf/voxelforge_perf.exe"

cargo_count() {
  powershell -NoProfile -Command \
    "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\").Count" \
    2>/dev/null | tr -d '[:space:]'
}

if [ -f "$EXE" ]; then
  { echo "ALREADY_BUILT $(date '+%H:%M:%S' 2>/dev/null)"; } > "$LOG"
  exit 0
fi

for attempt in 1 2 3 4 5; do
  # Wait for a clear build slot.
  while [ "$(cargo_count)" != "0" ]; do sleep 30; done
  sleep 8                       # settle, then re-check (close the race window)
  if [ "$(cargo_count)" != "0" ]; then continue; fi

  : > "$LOG"
  { echo "ATTEMPT $attempt start=$(date '+%H:%M:%S' 2>/dev/null)"; } >> "$LOG"
  cargo build -p voxelforge --bin voxelforge_perf --profile perf \
    --target-dir target-rose -j 8 >> "$LOG" 2>&1
  echo "BUILD_EXIT=$?" >> "$LOG"

  if [ -f "$EXE" ]; then
    echo "PROBE_BUILD_OK attempt=$attempt exe=present" >> "$LOG"
    exit 0
  fi

  # Stop on a real compile error (don't loop forever on a source bug).
  if grep -qiE 'error\[|^error|cannot find' "$LOG" && \
     ! grep -qi 'STATUS_DLL_INIT_FAILED\|0xc0000142' "$LOG"; then
    echo "PROBE_REAL_ERROR attempt=$attempt — stopping, Rose must read $LOG" >> "$LOG"
    exit 0
  fi
  echo "no exe after attempt $attempt (contention death?) — retry after slot clears" >> "$LOG"
  sleep 30
done
echo "PROBE_BUILD_EXHAUSTED_RETRIES $(date '+%H:%M:%S' 2>/dev/null)" >> "$LOG"
