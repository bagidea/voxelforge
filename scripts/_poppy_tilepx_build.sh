#!/usr/bin/env bash
# ===========================================================================
# Poppy — release build for the manifest-driven TILE_PX pass, in this lane's
# OWN target dir so it cannot collide with whoever else is building.
#
# The log ends with a `BUILD_EXIT=<code>` line, always. That line is the only
# thing that says a build finished: a `tasklist` full of rustc.exe can just as
# easily be somebody else's lane, and `tail` on a cargo log shows `Compiling`
# long after the first error.  Judge with:
#     grep -c '^error' <log>      and     grep 'BUILD_EXIT=' <log>
# ===========================================================================
set -u
cd "$(dirname "$0")/.."

LOG="${1:-_poppy_tilepx_build.log}"
: > "$LOG"
echo "=== START $(date '+%Y-%m-%dT%H:%M:%S') ===" >> "$LOG"

CARGO_TERM_COLOR=never cargo build --release \
  --bin voxelforge --target-dir target-poppy >> "$LOG" 2>&1
rc=$?

echo "=== EXIT $rc $(date '+%Y-%m-%dT%H:%M:%S') ===" >> "$LOG"
echo "BUILD_EXIT=$rc" >> "$LOG"
echo "ERROR_LINES=$(grep -c '^error' "$LOG")" >> "$LOG"
