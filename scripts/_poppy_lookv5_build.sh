#!/usr/bin/env bash
# Look-v5 build (poppy) — queue behind the box, then build, then mark done.
#
# WHY IT QUEUES INSTEAD OF JUST BUILDING. Three concurrent cargo builds on this
# box is how this repo gets `0xc0000142 STATUS_DLL_INIT_FAILED` out of a
# build-script exe (killed twice, 2026-07-31 20:39 and 20:50); the crash is
# commit headroom, not a code bug, and `-j 1` does not help because the peak
# comes from the NUMBER of builds, not the number of jobs.
#
# WHY NOT `_poppy_wait_for_free_box.sh`, WHICH WANTS ZERO. Two builds were
# already coexisting on this box when this one was queued (monanisa's
# target-monanisa and rose's target-rose, both healthy), so two is the level
# this machine is demonstrably holding. This waits for a free SLOT — at most one
# other build — rather than for an empty box, which on a five-lane afternoon can
# mean never.
#
# Counts LANES, not processes: every cargo invocation shows up as two cargo.exe
# (the ~/.cargo shim and the toolchain binary), so the unique `--target-dir` /
# `CARGO_TARGET_DIR` is what identifies a build.
set -u
ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
LOG="$ROOT/_poppy_lookv5_build.log"
DONE="$ROOT/_poppy_lookv5_build.done"
rm -f "$DONE"

other_lanes() {
  powershell.exe -NoProfile -Command \
    "(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\" |
      ForEach-Object { if (\$_.CommandLine -match 'target-[a-z]+') { \$Matches[0] } } |
      Where-Object { \$_ -ne 'target-poppy' } |
      Sort-Object -Unique |
      Measure-Object).Count" 2>/dev/null | tr -d '\r' | tr -d ' '
}

echo "=== [1/2] queue: waiting for a free build slot (<= 1 other lane) ==="
for i in $(seq 1 180); do   # 180 x 20s = 60 min cap
  n=$(other_lanes)
  case "$n" in ''|*[!0-9]*) n=9 ;; esac
  if [ "$n" -le 1 ]; then
    echo "  SLOT_FREE after $((i * 20))s — $n other lane(s) building"
    break
  fi
  printf '  %ss  other_lanes=%s\n' "$((i * 20))" "$n"
  sleep 20
done

echo "=== [2/2] cargo build --bin voxelforge --profile perf -j 2 ==="
# --profile perf, not the default dev profile: `_poppy_lookv4_chain.sh` shoots
# `target-poppy/perf/voxelforge.exe`, and a dev-profile Bevy build renders this
# 9537-block map at seconds-per-frame, which the capture harness cannot use.
# Same profile as the v4 plates, so the v5 pair straddles ONLY the look change.
cd "$ROOT/client" || exit 1
CARGO_TARGET_DIR="$ROOT/target-poppy" \
  cargo build --bin voxelforge --profile perf -j 2 > "$LOG" 2>&1
echo "EXIT=$?" > "$DONE"
cat "$DONE"
