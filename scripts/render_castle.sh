#!/usr/bin/env bash
# Load the generated castle map into the real game and prove it three ways:
#   1. overview  — a fly-cam beauty pass (screenshot of the whole castle)
#   2. walk      — drop a grounded player in the courtyard; land + wall-collision proof
# Both use the SAME voxelforge bin the player runs, headless via VOXELFORGE_SHOT.
set -u
EXE=target/release/voxelforge.exe
MAP=maps/castle.json

echo "=== overview ==="
env VOXELFORGE_MAP_LOAD="$MAP" VOXELFORGE_SHOT=castle_overview.png "$EXE" \
  >castle_overview.log 2>&1
echo "overview exit=$? ($(grep -c '' castle_overview.log) log lines)"
grep -E 'MAP_LOAD|MAP_APPLY|SHOT' castle_overview.log

echo "=== first-person walk (land + wall collision) ==="
env VOXELFORGE_MAP_LOAD="$MAP" VOXELFORGE_WALK_DEMO=1 VOXELFORGE_SHOT=castle_walk.png "$EXE" \
  >castle_walk.log 2>&1
echo "walk exit=$?"
grep -E 'MAP_LOAD|MAP_APPLY|WALK_DEMO|WALK_WALL|SHOT' castle_walk.log
