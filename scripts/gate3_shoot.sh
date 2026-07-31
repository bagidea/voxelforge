#!/usr/bin/env bash
# Gate 3 shoot — guard against a stale release binary, then capture 3 frames
# (boot / walk / combat) through the LookPlugin post stack at High quality.
#
# DESIGN:
#   This script DOES NOT build. It checks once that `target/release/voxelforge.exe`
#   is newer than the last commit to touch `client/src/look.rs` (i.e. it was
#   actually built from the current LookPlugin source), then immediately fires
#   the same three screenshot invocations a reviewer would run — one binary,
#   three modes, no recompile between them. There is nothing that rebuilds the
#   exe out from under this script, so the guard is a single pass/fail check,
#   not a poll loop: if the exe is stale, waiting longer will not fix it.
#   Every shot goes through `screenshot_once` (main.rs t=3.2s → save PNG →
#   AppExit at 4.4s), so the frames are deterministic.
#
#   Captures use env vars (not CLI flags) so a single script invocation owns the
#   whole environment and there is no shell-state leak between shots.
#
# ENV OVERRIDES (for Poppy / manual runs):
#   BIN           path to the voxelforge binary   (default: ./target/release/voxelforge.exe)
#   QUALITY       LookQuality tier                 (default: high)
#   OUT           output directory                 (default: docs/assets)
#   LOGS          log directory                    (default: _gate3_logs)
#   MIN_MTIME     epoch seconds the exe must be newer than (default: commit
#                 time of the latest commit touching client/src/look.rs, or
#                 that file's on-disk mtime if it has uncommitted edits)
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

look_rs_min_mtime() {
  # The guard's real intent: the exe must not be older than the LookPlugin
  # source it's supposed to represent. Use the newer of (a) the commit time of
  # the latest commit touching client/src/look.rs and (b) that file's current
  # on-disk mtime, so uncommitted edits also count.
  local commit_t=0 disk_t=0
  commit_t=$(git log -1 --format=%ct -- client/src/look.rs 2>/dev/null || echo 0)
  [ -f client/src/look.rs ] && disk_t=$(stat -c %Y client/src/look.rs 2>/dev/null || date -r client/src/look.rs +%s 2>/dev/null || echo 0)
  if [ "$disk_t" -gt "$commit_t" ]; then echo "$disk_t"; else echo "$commit_t"; fi
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

# ── guard: exe must not be older than the newest source file ────────────────

echo "=== GATE3 SHOOT — checking binary freshness ==="
echo "  bin:      $BIN"
echo "  quality:  $QUALITY"
echo "  output:   $OUT"

MTIME=$(exe_mtime)
MIN_MTIME="${MIN_MTIME:-$(newest_source_mtime)}"

fmt_t() { date -d "@$1" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -r "$1" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo "epoch $1"; }

echo "  exe mtime:      $MTIME ($(fmt_t "$MTIME"))"
echo "  newest source:  $MIN_MTIME ($(fmt_t "$MIN_MTIME"))"

if [ "${GATE3_SKIP_WAIT:-0}" = "1" ]; then
  echo "  ⚡ GATE3_SKIP_WAIT=1 — skipping freshness check"
elif [ "$MTIME" -eq 0 ]; then
  echo ""
  echo "  ✗ FAIL — $BIN does not exist"
  exit 1
elif [ "$MTIME" -lt "$MIN_MTIME" ]; then
  echo ""
  echo "  ✗ FAIL — exe is older than the newest source file."
  echo "    exe mtime=$MTIME < source mtime=$MIN_MTIME. Rebuild target/release/voxelforge.exe"
  echo "    (or pass MIN_MTIME to override, or GATE3_SKIP_WAIT=1 to skip)."
  exit 1
fi

echo ""
echo "  ✓ exe is fresh enough — proceeding immediately, no sleep"

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
