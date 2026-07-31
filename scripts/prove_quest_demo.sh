#!/usr/bin/env bash
# prove_quest_demo.sh — harness for `--quest-demo` proof run.
#
# Wraps the game output in a machine-readable PROOF envelope so every run is
# reproducible and gradeable. The gate is 4 grep lines:
#
#   QUEST_ACCEPT id=q3_gatekeeper
#   QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS
#   QUEST_COMPLETE q3_gatekeeper => PASS
#   QUEST_NEXT_OPEN q4_what_walls_remember => PASS
#
# All 4 present + exit 0 = PASS. Missing any = FAIL.
#
# Usage:
#   BIN=target-int/debug/voxelforge.exe bash scripts/prove_quest_demo.sh
#   bash scripts/prove_quest_demo.sh                         # default: target-quest
#
# Unlike prove_quest.sh, this script does NOT use `timeout` — the game
# controls its own exit via AppExit. `timeout` on Windows Git Bash maps to
# the wrong binary and silently returns 127 on a real run.
set -uo pipefail

BIN=${BIN:-./target-quest/debug/voxelforge.exe}
LOGDIR=_quest_proof
mkdir -p "$LOGDIR"

TS=$(date '+%Y-%m-%d %H:%M:%S')
LOG="$LOGDIR/quest-demo-PROOF.log"

# ---- proof envelope header ----
{
  echo "PROOF_START $TS"
  echo "BINARY=$BIN"
  if [ -f "$BIN" ]; then
    MTIME=$(stat -c '%Y' "$BIN" 2>/dev/null || stat -f '%m' "$BIN" 2>/dev/null || echo "unknown")
    echo "BINARY_MTIME=$MTIME"
  else
    echo "BINARY_MTIME=MISSING"
  fi
  echo "=== QUEST_DEMO_OUTPUT ==="
} > "$LOG"

# ---- copy audio assets if missing (Bevy resolves relative to exe) ----
exe_dir="$(dirname "$BIN")"
if [ ! -d "$exe_dir/assets/audio" ]; then
  mkdir -p "$exe_dir/assets"
  cp -r assets/audio "$exe_dir/assets/audio" 2>/dev/null || true
fi

# ---- run the demo ----
# Direct execution, no timeout wrapper — the game's quest_demo writes AppExit.
"$BIN" --quest-demo >>"$LOG" 2>&1
rc=$?

# ---- proof envelope footer ----
{
  echo "=== EXIT_CODE=$rc ==="
  echo "PROOF_END $(date '+%Y-%m-%d %H:%M:%S')"
} >> "$LOG"

# ---- grade ----
echo ""
echo "=== GRADE ==="
echo "binary: $BIN"
echo "exit code: $rc"
echo ""

PASS=true

check_line() {
  local label="$1" pattern="$2"
  if grep -q "$pattern" "$LOG"; then
    echo "  ✓ $label"
  else
    echo "  ✗ $label — NOT FOUND"
    PASS=false
  fi
}

check_line "QUEST_ACCEPT id=q3_gatekeeper"                     "QUEST_ACCEPT id=q3_gatekeeper"
check_line "QUEST_STAGE_COMPLETE o3_defeat => PASS"            "QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS"
check_line "QUEST_COMPLETE q3_gatekeeper => PASS"              "QUEST_COMPLETE q3_gatekeeper => PASS"
check_line "QUEST_NEXT_OPEN q4_what_walls_remember => PASS"    "QUEST_NEXT_OPEN q4_what_walls_remember => PASS"

# Exit code check
if [ "$rc" -ne 0 ]; then
  echo "  ✗ EXIT_CODE=$rc (expected 0)"
  PASS=false
else
  echo "  ✓ EXIT_CODE=0"
fi

# Report death if present
echo ""
if grep -q "COMBAT player died" "$LOG"; then
  echo "--- COMBAT DEATH DETECTED ---"
  grep -nE "COMBAT player died|RESPAWN|QUEST_KILL player died|player_hurt" "$LOG" || true
fi

# Report any FAIL lines
if grep -qE '=> FAIL|QUEST_FATAL' "$LOG"; then
  echo "--- FAIL / FATAL LINES ---"
  grep -nE '=> FAIL|QUEST_FATAL' "$LOG" || true
fi

echo ""
if $PASS; then
  echo "PROVE_QUEST: PASS — all 4 gates present, exit 0"
  exit 0
else
  echo "PROVE_QUEST: FAIL — see missing/failing lines above"
  exit 1
fi
