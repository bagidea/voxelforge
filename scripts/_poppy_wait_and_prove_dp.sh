#!/usr/bin/env bash
# Wait for the integration lead's target-int binary, then grade dodge/parry on it.
# This lane never starts a build of its own (docs/LANES.md build lock).
BIN=./target-int/debug/voxelforge.exe
for i in $(seq 1 200); do
  live=$(tasklist 2>/dev/null | grep -ci 'cargo\.exe\|rustc\.exe')
  if [ -f "$BIN" ] && [ "$live" -eq 0 ]; then
    echo "[$(date +%H:%M:%S)] binary present and box quiet — grading"
    BIN="$BIN" bash scripts/prove_dodge_parry.sh
    echo "GATE_EXIT=$?"
    exit 0
  fi
  [ $((i % 10)) -eq 0 ] && echo "[$(date +%H:%M:%S)] waiting: bin=$([ -f "$BIN" ] && echo yes || echo no) cargo/rustc=$live"
  sleep 15
done
echo "[$(date +%H:%M:%S)] gave up waiting for $BIN"
exit 2
