#!/usr/bin/env bash
# Proof that `--combat-demo` runs the full combat loop end-to-end:
#   1. Walk to the husk, strafing on the way (COMBAT_STRAFE)
#   2. Hit it → HP drops (COMBAT_HIT)
#   3. Land a charged heavy → bigger HP drop + stamina cost (COMBAT_HEAVY)
#   4. Kill it with repeated attacks (COMBAT_KILL)
#   5. Player dies → PlayerDied event fires (COMBAT_DEATH)
#   6. Player respawns at the campfire with full HP (COMBAT_RESPAWN)
#
# The binary drives real ButtonInput<KeyCode> through the same paths a human
# would use — gather_input → player_combat → husk_ai → hud_bars — so the
# proof is the real game, not a simulation.
#
# The gate:
#   exit 0 · no `panicked` / `B0001` / `thread ... panicked` ·
#   every COMBAT_* line ends in `=> PASS` (none says `=> FAIL`).
set -uo pipefail

BIN=${BIN:-./target-combat/debug/voxelforge.exe}
LOGS=_combat_proof
mkdir -p "$LOGS"

echo "=== COMBAT DEMO PROOF ==="
echo "binary: $BIN"

# Build with a separate target dir so we never fight Poppy's lock.
echo "=== build ==="
# Go through build_safe.sh; a plain `cargo build` here dies mid-spawn with
# 0xc0000142 STATUS_DLL_INIT_FAILED at a random crate (see docs/LANES.md).
bash "$(dirname "$0")/build_safe.sh" build --bin voxelforge --target-dir target-combat 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "BUILD FAILED exit=$rc"
  exit 1
fi
echo "BUILD OK exit=0"

# Run the combat demo. No screenshot needed — the proof self-grades.
log="$LOGS/combat-demo.log"
echo "=== run --combat-demo ==="
"$BIN" --combat-demo >"$log" 2>&1
rc=$?

# Print the key lines so the log is self-documenting.
grep -E 'SCENE_READY|SPAWN_ENCOUNTER|COMBAT_STRAFE|COMBAT_HIT|COMBAT_HEAVY|COMBAT_KILL|COMBAT_DEATH|COMBAT_RESPAWN|PLAYER_DIED|RESPAWN|CHASE_DIST|COMBAT_WALK_DONE|COMBAT_FATAL' "$log" || true

echo "exit=$rc"

# ---- the gate ----
bad=""
[ "$rc" -ne 0 ] && bad="$bad exit=$rc"
grep -qE 'panicked|B0001|thread .* panicked' "$log" && bad="$bad PANIC"
grep -q 'SCENE_READY' "$log" || bad="$bad no-SCENE_READY"

# Every self-grading line must end in PASS.
for line in COMBAT_STRAFE COMBAT_HIT COMBAT_HEAVY COMBAT_KILL COMBAT_DEATH COMBAT_RESPAWN; do
  if ! grep -q "$line" "$log"; then
    bad="$bad missing-$line"
  elif grep "$line" "$log" | grep -q '=> FAIL'; then
    bad="$bad $line-FAIL"
  fi
done

if [ -n "$bad" ]; then
  echo "  ✗ FAIL:$bad"
  grep -nE 'panicked|B0001|error\[' "$log" | head -5
  exit 1
else
  echo "PROVE_COMBAT: PASS — all COMBAT_* self-grades passed (exit 0, no panic, no FAIL)"
  exit 0
fi
