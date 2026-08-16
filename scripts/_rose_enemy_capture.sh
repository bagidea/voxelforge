#!/usr/bin/env bash
# Rose — enemy AI before/after capture through the proof bin.
# Runs the lane's own voxelforge_enemyai_proof (links enemies.rs + enemy_ai.rs
# ONLY — never blocked by another lane's mid-edit file) against the scripted
# player, then picks the before/after behaviour pairs from the CSV trace it
# wrote. Usage: scripts/_rose_enemy_capture.sh [outdir] (default docs/assets/ai)
set -u
cd "$(dirname "$0")/.."
OUT="${1:-docs/assets/ai}"
FRAMES="$OUT/_frames"
BIN=target-rose/debug/voxelforge_enemyai_proof.exe

rm -rf "$FRAMES" && mkdir -p "$FRAMES" "$OUT"
VOXELFORGE_AIFRAMES="$FRAMES" VOXELFORGE_AILOG="$OUT/trace.csv" \
  timeout 150 "./$BIN" > "$OUT/runlog.txt" 2>&1
echo "exit=$?"
echo "--- proof verdict ---"
grep -E "AIPROOF (bin|cast|frame|DONE)" "$OUT/runlog.txt" | tail -5
echo "--- telegraph commits ---"
grep -c "TELEGRAPH->COMMIT" "$OUT/runlog.txt" || true
echo "--- pick before/after pairs from the trace ---"
python scripts/_rose_ai_pairs.py "$OUT/trace.csv" "$FRAMES" "$OUT"
ls -la "$OUT"/ai-[ab]-*.png
