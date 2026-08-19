#!/usr/bin/env bash
# Kevin — foliage-density pass: shoot the outdoor-noon plate + sample FPS.
#
# ONE binary, SAME camera, SAME hour. `before` and `after` differ only in the
# map on disk (maps/edhari.json), which the play scene loads at runtime — so a
# single perf build is an honest A/B. The FPS run reuses the identical
# VOXELFORGE_CINE + LOOK_* env as the still, so the FPS number prices the same
# frame composition the edge metric (scripts/_kevin_art_gap_measure.py) grades.
#
# Usage:  _kevin_foliage_shoot.sh [exe] [outdir] [before|after]
set -euo pipefail
ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
EXE="${1:-$ROOT/target-kevin/perf/voxelforge.exe}"
OUT="${2:-$ROOT/_kevin_foliage}"
MODE="${3:-before}"
mkdir -p "$OUT"

# Shared: playable scene, no HUD, top tier. Identical to _poppy_lookv3_shoot.cmd.
export VOXELFORGE_PLAY=1
export VOXELFORGE_NOHUD=1
export VOXELFORGE_LOOK_QUALITY=ultra
export VOXELFORGE_CINE_START=1.0
# outdoor-noon establishing shot (the shipped plate graded at edge 36.8).
export VOXELFORGE_CINE="44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
export VOXELFORGE_LOOK_SUN="66,205,20000"
export VOXELFORGE_LOOK_LIGHT="1.00,0.98,0.93,0.84,0.88,1.00"
export VOXELFORGE_LOOK_EXPOSURE=10.6

# 1) the still (t=3.2s, exits t=4.4s)
export VOXELFORGE_SHOT="$OUT/$MODE.png"
echo "=== [$MODE] SHOT -> $VOXELFORGE_SHOT ==="
"$EXE" --play

# 2) steady-state FPS over the same pose (warmup 3.2s, then 4s of samples).
#    Opt-in via VOXELFORGE_FPS_LEG=1: the 14-Aug target-kevin/perf exe predates
#    the VOXELFORGE_FPS_BENCH sampler (in source, not yet built), and without it
#    this leg never exits (no SHOT => no screenshot_once auto-exit). Skip by
#    default so the old binary can't hang.
if [ "${VOXELFORGE_FPS_LEG:-0}" = "1" ]; then
  unset VOXELFORGE_SHOT
  export VOXELFORGE_FPS_BENCH=4
  echo "=== [$MODE] FPS ==="
  "$EXE" --play
fi
