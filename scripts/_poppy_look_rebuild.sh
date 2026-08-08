#!/usr/bin/env bash
# Poppy — gate-0 rebuild for the eff6c7b look audit.
#
# Why this exists: the previous audit graded frames rendered by an exe from
# 2026-08-03 17:03, i.e. 27h older than the sky/exposure commit it claimed to
# measure. Every number was void. This script makes the build itself the
# evidence: a start stamp, the full build log, and the exe mtime afterwards.
#
# Judged by `grep -c '^error' build.log` == 0 AND exe mtime > start stamp.
# Never by a pipe exit code (tee swallows cargo's status on this box).
set -uo pipefail
cd "$(dirname "$0")/.."
OUT=_poppy_look
mkdir -p "$OUT"

# Never stack cargo builds here — a third concurrent one has died with
# STATUS_DLL_INIT_FAILED before. Wait the boom lane's build out first.
waited=0
while tasklist 2>/dev/null | grep -qi 'cargo\.exe'; do
  sleep 15
  waited=$((waited + 15))
  echo "waiting for the other cargo build... ${waited}s" >> "$OUT/rebuild.trace"
done
echo "cargo lane free after ${waited}s at $(date '+%Y-%m-%d %H:%M:%S')" >> "$OUT/rebuild.trace"

# Only `voxelforge` — the client package ships three bins and every one of them
# links with fat LTO, so `cargo build --release` alone would pay that link three
# times for two artifacts nothing in this audit renders.
date '+%Y-%m-%d %H:%M:%S' | tee "$OUT/build_start.stamp"
cargo build --release --bin voxelforge 2>&1 | tee build.log
date '+%Y-%m-%d %H:%M:%S' | tee "$OUT/build_end.stamp"

{
  echo "start:      $(cat "$OUT/build_start.stamp")"
  echo "end:        $(cat "$OUT/build_end.stamp")"
  echo "errors:     $(grep -c '^error' build.log)"
  echo "warnings:   $(grep -c '^warning' build.log)"
  echo "exe:        $(ls -la --time-style='+%Y-%m-%d %H:%M:%S' target/release/voxelforge.exe 2>&1)"
} | tee "$OUT/gate0.txt"
