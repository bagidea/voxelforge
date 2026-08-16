#!/usr/bin/env bash
# _pixel_ai_check.sh — enemy-AI lane compile gate.
#
# Per docs/LANES.md "The build lock": a lane verifies its own work with
# `cargo check` into its OWN warm target dir, never a full `cargo build`
# into the shared `target/`. This one uses `target-combat` (the combat
# lane's dir, which this lane owns for the enemy-AI pass).
#
# Detached-friendly: writes everything to a log + a DONE stamp so a caller
# never has to sit on the process. Grades cargo's REAL exit code (cargo is
# never piped — see build_safe.sh's PIPE warning).
set -uo pipefail
# Launched detached via Start-Process, bash inherits a PowerShell PATH with no
# /usr/bin on it — dirname/rm/date all vanish and the script silently writes to
# the wrong drive. Put the Git-for-Windows tools back before anything else.
export PATH="/usr/bin:/bin:$PATH"
cd "$(dirname "$0")/.."

LOG="${1:-_pixel_ai_check.log}"
STAMP="${LOG%.log}.done"
rm -f "$STAMP"

export CARGO_TARGET_DIR="target-combat"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_INCREMENTAL=0

{
  echo "=== _pixel_ai_check start $(date '+%F %T') ==="
  echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR jobs=$CARGO_BUILD_JOBS"
} > "$LOG"

cargo check -p voxelforge --bin voxelforge --message-format short >> "$LOG" 2>&1
rc=$?
echo "=== CHECK EXIT $rc ===" >> "$LOG"

# Unit tests compile too — the perception/tactics pass ships four of them.
cargo check -p voxelforge --bin voxelforge --tests --message-format short >> "$LOG" 2>&1
rc2=$?
echo "=== CHECK-TESTS EXIT $rc2 ===" >> "$LOG"

echo "check=$rc tests=$rc2" > "$STAMP"
echo "=== _pixel_ai_check done $(date '+%F %T') rc=$rc/$rc2 ===" >> "$LOG"
exit "$rc"
