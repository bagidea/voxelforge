#!/usr/bin/env bash
# Poppy — the two new voxel.rs tests, run against the SAME release profile the
# shipped binary uses (deps are already built in target-poppy, so this only
# relinks the bin as a test harness). Ends with TEST_EXIT=<code>, always: that
# line is the only thing that says the run finished.
set -u
cd "$(dirname "$0")/.."
LOG="${1:-_poppy_tilepx_test.log}"
: > "$LOG"
echo "=== START $(date '+%Y-%m-%dT%H:%M:%S') ===" >> "$LOG"
CARGO_TERM_COLOR=never cargo test --release --bin voxelforge --target-dir target-poppy \
  -- --nocapture manifest >> "$LOG" 2>&1
rc=$?
echo "=== EXIT $rc $(date '+%Y-%m-%dT%H:%M:%S') ===" >> "$LOG"
echo "TEST_EXIT=$rc" >> "$LOG"
echo "ERROR_LINES=$(grep -c '^error' "$LOG")" >> "$LOG"
