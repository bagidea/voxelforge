#!/usr/bin/env bash
# Enemy-archetype chase clip (Rose, 2026-08-14).
#
# Runs the isolated proof bin (voxelforge_enemyai_proof — links enemies.rs +
# enemy_ai.rs only, no combat.rs) which renders Monanisa's three bodies driven
# by the three archetypes against a scripted 4-phase player route, capturing
# numbered PNGs + a per-frame CSV trace, then assembles the clip with ffmpeg.
#
# The bin prints its own AI transitions ("AI id=<n> <archetype> <from> -> <to>")
# — this script only tees them, never prints verdicts itself.
#
# Usage: bash scripts/_rose_ai_clip.sh <path-to-voxelforge_enemyai_proof.exe>
set -euo pipefail

EXE="${1:?usage: _rose_ai_clip.sh <voxelforge_enemyai_proof.exe>}"
OUT="docs/assets/ai"
FRAMES="$OUT/_archetype_frames"
mkdir -p "$FRAMES"

VOXELFORGE_AIFRAMES="$FRAMES" \
VOXELFORGE_AILOG="$OUT/trace_archetypes.csv" \
  "$EXE" 2>&1 | tee "$OUT/runlog_archetypes.txt" | tail -5

ffmpeg -y -framerate 30 -i "$FRAMES/f%04d.png" -c:v libx264 -pix_fmt yuv420p \
  -crf 23 "$OUT/enemy-archetypes.mp4" 2>&1 | tail -2

echo "clip: $OUT/enemy-archetypes.mp4 ($(stat -c %s "$OUT/enemy-archetypes.mp4") bytes)"
