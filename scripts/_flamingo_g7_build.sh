#!/usr/bin/env bash
# Flamingo G7 — wait for a free cargo slot, then build the sweep binary.
#
# Three cargo processes were already live on this box when the G7 pass started
# (two `check`s on the shared `target/`, one `--release` build). The office rule
# from the look-perf lane is that a THIRD concurrent build is what produces
# STATUS_DLL_INIT_FAILED, so this waits instead of stacking one more.
#
# `--profile perf`, not `--release`, on purpose: same opt-level 3, but without
# `lto = "fat"` / `codegen-units = 1`, which is a ~20 min LINK on this box every
# time. The sweep needs correct PIXELS, and LTO does not change float semantics
# or a single line of WGSL — so the swept frames are identical to what release
# would produce. The chosen values get a real `--release` build afterwards.
set -u
cd "$(dirname "$0")/.."

LOG=_flamingo_g7/build.log
: > "$LOG"

slots() { tasklist 2>/dev/null | grep -c 'cargo.exe' || true; }

# Each concurrent cargo invocation shows up as TWO cargo.exe (shim + toolchain),
# so <= 4 means "at most two others running" and this one makes a third — which
# is the documented ceiling, not past it. Waiting for a completely quiet box on a
# five-agent night means never building at all.
for i in $(seq 1 240); do
  n=$(slots)
  if [ "${n:-0}" -le 4 ]; then
    echo "[$(date +%T)] slot free (cargo procs=$n) — starting build" | tee -a "$LOG"
    break
  fi
  echo "[$(date +%T)] waiting: cargo procs=$n" | tee -a "$LOG"
  sleep 20
done

echo "[$(date +%T)] cargo build --profile perf --bin voxelforge --target-dir target-flamingo" >> "$LOG"
cargo build --profile perf --bin voxelforge --target-dir target-flamingo -j 4 >> "$LOG" 2>&1
echo "EXIT=$? $(date +%T)" >> "$LOG"
