#!/usr/bin/env bash
# act1_shots.sh — capture one still per Act-1 story beat, in-engine.
#
# Companion to act1_runtime_proof.sh: the proof grades the quest loop from the
# engine's own stdout; this script runs ONE ordinary in-order pass and asks the
# engine to save a PNG the moment each beat completes. Every capture is queued
# by the game (quest.rs `shoot_beat` / `check_act_end`), never by this harness —
# this script only sets the env knobs and copies assets.
#
# Beats (6): q1 campfire, q2 gate, q3 gatekeeper, q4 walls, q5 sigil, act_end.
#
# Usage:
#   bash scripts/act1_shots.sh
#   BIN=target-sun/debug/voxelforge.exe bash scripts/act1_shots.sh
#
# Exit: 0 shots queued (check the PNGs + log below) · 2 binary not ready.

set -uo pipefail

BIN=${BIN:-./target-sun/debug/voxelforge.exe}
SHOTDIR=${SHOTDIR:-docs/assets/story}
LOG=${LOG:-_quest_proof/act1-shots.log}

if [ ! -f "$BIN" ]; then
  echo "NOT_READY — $BIN does not exist. Build first."
  exit 2
fi

# Bevy resolves audio/story relative to the exe; the map is cwd-relative
# (scene.rs `play_map()`), so run from the repo root exactly like the proof.
exe_dir="$(dirname "$BIN")"
for d in audio story; do
  if [ ! -d "$exe_dir/assets/$d" ] && [ -d "assets/$d" ]; then
    mkdir -p "$exe_dir/assets"
    cp -r "assets/$d" "$exe_dir/assets/$d" 2>/dev/null || true
  fi
done

mkdir -p "$SHOTDIR" "$(dirname "$LOG")"

echo "=== ACT1 SHOTS ==="
echo "binary: $BIN"
echo "shotdir: $SHOTDIR   log: $LOG"

# `none` is the empty chaos env var (see act1_runtime_proof.sh), i.e. leave it unset.
VOXELFORGE_QUEST_SHOTS="$SHOTDIR" \
VOXELFORGE_ACT1_SHOT="$SHOTDIR/act1_q6_act_end.png" \
  "$BIN" --quest-demo > "$LOG" 2>&1
rc=$?

echo "process exit: $rc"
echo "--- engine shot queue (QUEST_SHOT / ACT1_SHOT lines) ---"
grep -aE 'QUEST_SHOT|ACT1_SHOT' "$LOG" || echo "  (none — see $LOG)"
echo "--- on disk ---"
ls -1 "$SHOTDIR" 2>/dev/null | sed 's/^/  /' || true
echo ""
if [ "$rc" -eq 0 ]; then
  echo "ACT1_SHOTS: done — verify every beat PNG is non-trivial before committing."
else
  echo "ACT1_SHOTS: process exited $rc — inspect $LOG."
fi
exit "$rc"
