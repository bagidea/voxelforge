#!/usr/bin/env bash
# Proof that `--quest-demo` runs the full quest loop end-to-end:
#   1. Walk to the gate zone (quest q1 auto-completes from approach trigger)
#   2. Interact with Elder Maren at the gate → accept quest (QUEST_ACCEPT)
#   3. Walk to guard post east → kill Garren the Husk (QUEST_KILL)
#   4. Quest completes → reward received (QUEST_COMPLETE)
#   5. Next quest unlocks (QUEST_NEXT_OPEN)
#
# The binary drives real ButtonInput<KeyCode> through the same paths a human
# would use — gather_input → player_combat → husk_ai → quest system — so the
# proof is the real game, not a simulation.
#
# The gate:
#   exit 0 · no `panicked` / `B0001` / `thread ... panicked` ·
#   every QUEST_* line ends in `=> PASS` (none says `=> FAIL`).
set -uo pipefail

BIN=${BIN:-./target-quest/debug/voxelforge.exe}
LOGS=_quest_proof
mkdir -p "$LOGS"

echo "=== QUEST DEMO PROOF ==="
echo "binary: $BIN"

# Build with a separate target dir so we never fight Poppy's lock.
echo "=== build ==="
bash "$(dirname "$0")/build_safe.sh" build --bin voxelforge --target-dir target-quest 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "BUILD FAILED exit=$rc"
  exit 1
fi
echo "BUILD OK exit=0"

# Run the quest demo. The proof self-grades.
log="$LOGS/quest-demo.log"
echo "=== run --quest-demo ==="

# Ensure audio assets are reachable from the build output directory (Bevy
# resolves asset paths relative to the exe, not the project root).
exe_dir="$(dirname "$BIN")"
if [ ! -d "$exe_dir/assets/audio" ]; then
  mkdir -p "$exe_dir/assets"
  cp -r assets/audio "$exe_dir/assets/audio"
  echo "ASSETS_COPY audio/ -> $exe_dir/assets/audio"
fi

# Wrap with timeout (180 s) so a hung binary is killed and logged as a clear
# FAIL, not a silent stall that kills the caller session.
timeout 180 "$BIN" --quest-demo >"$log" 2>&1
rc=$?
if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
  echo "=> FAIL (timeout)" >>"$log"
fi

# Print the key lines so the log is self-documenting.
grep -E 'SCENE_READY|SPAWN_NPC|QUEST_INIT|QUEST_ACCEPT|QUEST_KILL|QUEST_STAGE_COMPLETE|QUEST_COMPLETE|QUEST_NEXT_OPEN|QUEST_PROOF|QUEST_FATAL|STORY_LOAD|QUEST_INTERACT|QUEST_DIALOGUE|QUEST_WALK|QUEST_AREA|QUEST_CHECK' "$log" || true

echo "exit=$rc"

# ---- the gate ----
bad=""
[ "$rc" -ne 0 ] && bad="$bad exit=$rc"
grep -qE 'panicked|B0001|thread .* panicked' "$log" && bad="$bad PANIC"
grep -q 'SCENE_READY' "$log" || bad="$bad no-SCENE_READY"
grep -q 'STORY_LOAD ok' "$log" || bad="$bad no-STORY_LOAD"

# Every self-grading line must end in PASS for the gates that were reached.
# Some gates may be skipped (e.g. if q1 auto-completes before we log it),
# but the critical ones must all pass.
for line in QUEST_ACCEPT QUEST_KILL QUEST_STAGE_COMPLETE QUEST_COMPLETE QUEST_NEXT_OPEN; do
  if ! grep -q "$line" "$log"; then
    bad="$bad missing-$line"
  elif grep "$line" "$log" | grep -q '=> FAIL'; then
    bad="$bad $line-FAIL"
  fi
done

# The final summary line must say PASS.
if ! grep -q 'QUEST_PROOF.*PASS' "$log"; then
  bad="$bad QUEST_PROOF-not-PASS"
fi

if [ -n "$bad" ]; then
  echo "  ✗ FAIL:$bad"
  grep -nE 'panicked|B0001|error\[|FAIL' "$log" | head -10
  exit 1
else
  echo "PROVE_QUEST: PASS — all QUEST_* self-grades passed (exit 0, no panic, no FAIL)"
  exit 0
fi
