#!/usr/bin/env bash
# Poppy — A1 dome-dead close-out: build the worktree copy of the dome source.
#
# Builds from the DETACHED WORKTREE (_vf_poppy_dome), never the shared checkout,
# because look.rs there carries other lanes' uncommitted work (Flamingo holds the
# exposure/ambient/fog knobs this session and must not be dragged into my link).
#
# CARGO_TARGET_DIR is the lane's own target-poppy — the central target/ is off
# limits, and a fresh dep tree costs ~25min of Bevy for no new information.
#
# Queues behind the box the same way `_poppy_lookv6_build.sh` does: three
# concurrent cargo builds is how this repo earns 0xc0000142 out of a build-script
# exe. Counts LANES (unique --target-dir), not cargo.exe processes.
set -u
WT="E:/Projects/bagidea-ai-agents-office/workspace/projects/_vf_poppy_dome"
ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
LOG="$ROOT/_poppy_dome_build.log"
DONE="$ROOT/_poppy_dome_build.done"
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
for i in $(seq 1 180); do
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
# --profile perf: the dev profile renders this map at seconds-per-frame and the
# capture harness times out on it. Same profile every look plate was shot on.
cd "$WT/client" || exit 1
CARGO_TARGET_DIR="$ROOT/target-poppy" \
  cargo build --bin voxelforge --profile perf -j 2 > "$LOG" 2>&1
echo "EXIT=$?" > "$DONE"
cat "$DONE"
