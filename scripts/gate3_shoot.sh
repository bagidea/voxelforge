#!/usr/bin/env bash
# Gate 3 shoot — wait for a fresh release binary, then capture 3 frames
# (boot / walk / combat) through the LookPlugin post stack at High quality.
#
# DESIGN:
#   This script DOES NOT build. It waits for `target/release/voxelforge.exe` to
#   appear with an mtime newer than "right now", then fires the same three
#   screenshot invocations a reviewer would run — one binary, three modes, no
#   recompile between them. Every shot goes through `screenshot_once` (main.rs
#   t=3.2s → save PNG → AppExit at 4.4s), so the frames are deterministic.
#
#   Captures use env vars (not CLI flags) so a single script invocation owns the
#   whole environment and there is no shell-state leak between shots.
#
# ENV OVERRIDES (for Poppy / manual runs):
#   BIN           path to the voxelforge binary   (default: ./target/release/voxelforge.exe)
#   QUALITY       LookQuality tier                 (default: high)
#   OUT           output directory                 (default: docs/assets)
#   LOGS          log directory                    (default: _gate3_logs)
#   WAIT_TIMEOUT  max seconds to wait for the exe  (default: 600 = 10 min)
#   POLL_SEC      how often to check mtime         (default: 5)
#
# Usage:
#   bash scripts/gate3_shoot.sh              # wait + shoot all 3
#   QUALITY=ultra bash scripts/gate3_shoot.sh  # full hero stack
#   BIN=./target/debug/voxelforge.exe bash scripts/gate3_shoot.sh  # debug binary
#
# Gate (mechanical, no by-eye grading):
#   Every shot must: exit 0 · PNG on disk & ≥ 2 KiB · "SHOT saved" in log ·
#   no `panicked` / `B0001` / `thread ... panicked` anywhere.
set -euo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
QUALITY="${QUALITY:-high}"
OUT="${OUT:-docs/assets}"
LOGS="${LOGS:-_gate3_logs}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-600}"
POLL_SEC="${POLL_SEC:-5}"

mkdir -p "$OUT" "$LOGS"

# ── helpers ──────────────────────────────────────────────────────────────────

now_epoch() {
  date +%s
}

exe_mtime() {
  if [ -f "$BIN" ]; then
    stat -c %Y "$BIN" 2>/dev/null || date -r "$BIN" +%s 2>/dev/null || echo 0
  else
    echo 0
  fi
}

gate_one_shot() {
  # $1 = output PNG path, $2 = label for log, $3+ = env vars to set before the run
  local png="$1"; local label="$2"; shift 2
  local log="$LOGS/${label}.log"

  echo "=== GATE3: $label  ($png) ==="
  echo "  quality=$QUALITY  bin=$BIN"

  # Build the env preamble: each positional arg after label is KEY=VALUE.
  local env_preamble=()
  for kv in "$@"; do
    env_preamble+=("$kv")
  done
  env_preamble+=("VOXELFORGE_SHOT=$png")
  env_preamble+=("VOXELFORGE_LOOK_QUALITY=$QUALITY")

  # Run the binary; capture stdout+stderr together.
  env "${env_preamble[@]}" "$BIN" >"$log" 2>&1 || true
  local rc=${PIPESTATUS[0]:-$?}

  # Always print the key lines so the log is self-documenting.
  grep -E 'MAP_LOAD|MAP_APPLY|SPAWN_ENCOUNTER|SCENE_READY|SHOT saved|LOOK' "$log" || true

  # ── gate checks ──────────────────────────────────────────────────────────
  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  exit=$rc  bytes=$bytes"

  local bad=""
  [ "$rc" -ne 0 ]                           && bad="$bad exit=$rc"
  [ "$bytes" -lt 2048 ]                     && bad="$bad no-png"
  grep -qE 'panicked|B0001' "$log"          && bad="$bad PANIC"
  grep -q 'SHOT saved'   "$log"             || bad="$bad no-SHOT"

  if [ -n "$bad" ]; then
    echo "  ✗ FAIL:$bad"
    grep -nE 'panicked|B0001|error\[' "$log" | head -5
    return 1
  else
    echo "  ✓ PASS"
    return 0
  fi
}

# ── wait for the binary ──────────────────────────────────────────────────────

echo "=== GATE3 SHOOT — waiting for fresh binary ==="
echo "  bin:      $BIN"
echo "  quality:  $QUALITY"
echo "  output:   $OUT"
echo "  timeout:  ${WAIT_TIMEOUT}s"

# Snapshot "now" in epoch seconds — we want an exe that is strictly newer.
DEADLINE=$(($(now_epoch) + WAIT_TIMEOUT))
SNAPSHOT=$(now_epoch)

echo "  snapshot: $SNAPSHOT ($(date -d "@$SNAPSHOT" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -r "$SNAPSHOT" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo "epoch $SNAPSHOT"))"
echo "  deadline: $DEADLINE"

waited=0
while true; do
  MTIME=$(exe_mtime)

  if [ "$MTIME" -gt "$SNAPSHOT" ]; then
    echo ""
    echo "  ✓ binary found — mtime=$MTIME > snapshot=$SNAPSHOT  (waited ${waited}s)"
    break
  fi

  if [ "$(now_epoch)" -ge "$DEADLINE" ]; then
    echo ""
    echo "  ✗ TIMEOUT after ${waited}s — binary mtime=$MTIME never passed snapshot=$SNAPSHOT"
    echo "    Is the release build still running? (check tasklist / cargo)"
    exit 1
  fi

  if [ "$waited" -eq 0 ]; then
    # First poll: report what we see right away so the log is useful.
    if [ "$MTIME" -eq 0 ]; then
      echo "  … binary not on disk yet — waiting (poll every ${POLL_SEC}s)"
    else
      echo "  … binary exists but mtime=$MTIME ≤ snapshot=$SNAPSHOT (not rebuilt yet)"
    fi
  elif [ $((waited % 30)) -eq 0 ]; then
    echo "  … still waiting (${waited}s elapsed, mtime=$MTIME)"
  fi

  sleep "$POLL_SEC"
  waited=$((waited + POLL_SEC))
done

# Small grace period: let the filesystem finish flushing the write. A build that
# just closed the file handle may still be syncing its last blocks on spinning
# rust / a busy CI runner.
sleep 2

# ── the three frames ─────────────────────────────────────────────────────────

echo ""
echo "=== shooting 3 frames ==="

FAILS=0

# (1) BOOT — the player spawns into the world; no input is pressed. Captures
#     the initial spawn pose with LookPlugin active (High tier: DOF, TAA, PCSS
#     penumbra, medium SSAO, distance haze).
gate_one_shot \
  "$OUT/gate3-after-boot.png" \
  "boot" \
  "VOXELFORGE_PLAY=1" \
  || FAILS=$((FAILS + 1))

# (2) WALK — the `--play-demo` path (`VOXELFORGE_PLAY_DEMO=1`) drives real
#     `ButtonInput` (W, D, left-click to grab cursor, MouseMotion to orbit the
#     camera) through the same controller a human uses, so by t=3.2s the avatar
#     has walked and the camera has orbited sideways — mid-stride frame.
gate_one_shot \
  "$OUT/gate3-after-walk.png" \
  "walk" \
  "VOXELFORGE_PLAY_DEMO=1" \
  || FAILS=$((FAILS + 1))

# (3) COMBAT — `--combat-demo` boots straight to AppState::Play with a husk
#     spawned 2.2 blocks ahead and the combat HUD on screen. The combat proof
#     drives the player toward the husk; at t=3.2s (1.7s into the walk phase)
#     the avatar is closing on the enemy with HUD bars visible.
gate_one_shot \
  "$OUT/gate3-after-combat.png" \
  "combat" \
  "VOXELFORGE_COMBAT_DEMO=1" \
  || FAILS=$((FAILS + 1))

# ── summary ──────────────────────────────────────────────────────────────────

echo ""
echo "=== GATE3 SHOOT — done ==="
echo "  quality:  $QUALITY"
echo "  failures: $FAILS / 3"

if [ "$FAILS" -eq 0 ]; then
  echo "GATE3_SHOOT: PASS — 3/3 frames clean"
  echo "  $OUT/gate3-after-boot.png"
  echo "  $OUT/gate3-after-walk.png"
  echo "  $OUT/gate3-after-combat.png"
  exit 0
else
  echo "GATE3_SHOOT: FAIL — $FAILS shot(s) bad (check $LOGS/*.log)"
  exit "$FAILS"
fi
