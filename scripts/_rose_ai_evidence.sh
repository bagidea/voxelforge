#!/usr/bin/env bash
# Enemy-AI before/after evidence runs (Rose, 2026-08-14).
#
# Runs the REAL game binary twice through the scripted AI exercise
# (combat.rs's own husk_ai_demo — the system prints its own AI_HUSK /
# AI_SQUAD / AI_DEMO lines; this script only captures stdout + the
# VOXELFORGE_AI_TRACE CSV + in-engine stills):
#   before = VOXELFORGE_AI_LEGACY=1  (proximity-mine aggro, beeline, no squad)
#   after  = default                 (perception + tactics pass)
# Then plots both traces side-by-side with scripts/_rose_ai_plot.py.
#
# Usage: bash scripts/_rose_ai_evidence.sh <path-to-voxelforge.exe>
set -euo pipefail

EXE="${1:?usage: _rose_ai_evidence.sh <voxelforge.exe>}"
OUT="docs/assets/ai"
mkdir -p "$OUT/shots_before" "$OUT/shots_after"

run () {  # run <tag> [extra env...]
  local tag="$1"; shift
  echo "=== AI evidence run: $tag ==="
  env "$@" \
    VOXELFORGE_PLAY=1 \
    VOXELFORGE_AI_DEMO=1 \
    VOXELFORGE_AI_LOG=1 \
    VOXELFORGE_AI_TRACE="$OUT/trace_$tag.csv" \
    VOXELFORGE_AI_SHOTS="$OUT/shots_$tag" \
    "$EXE" 2>&1 | tee "$OUT/runlog_$tag.txt" | grep -E "AI_MODE|AI_DEMO overall" || true
}

run before VOXELFORGE_AI_LEGACY=1
run after

python scripts/_rose_ai_plot.py "$OUT/paths_before_after.png" \
  --side-by-side "$OUT/trace_before.csv" "$OUT/trace_after.csv"

echo "=== evidence done: $OUT ==="
