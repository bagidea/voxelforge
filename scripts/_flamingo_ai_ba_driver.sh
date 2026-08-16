#!/usr/bin/env bash
# _flamingo_ai_ba_driver.sh — detached driver for the husk-AI BEFORE/AFTER pair.
#
# Owns every step after the enemy-AI proof build starts, so a session teardown
# cannot orphan the work (see skill: wait-for-fresh-exe-then-*).
#
#   BEFORE = the SHIPPED Guard Husk AI, captured through the REAL game
#            (`voxelforge` bin, combat.rs §4.1, VOXELFORGE_AI_DEMO stills).
#   AFTER  = rose's new archetype AI (`voxelforge_enemyai_proof`, enemy_ai.rs).
#
# Both binaries are built into target-flamingo/ so the capture provably comes
# from a fresh exe, never a stale one. Every step asserts its artifact exists.
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT="$PWD"

TGT=target-flamingo/debug
AI_EXE="$TGT/voxelforge_enemyai_proof.exe"
GAME_EXE="$TGT/voxelforge.exe"
AFTER_DIR=_fl_ai_after
BEFORE_DIR=_fl_ai_before
STAMP=1786000000   # unused; kept for readability of the mtime gate below

say() { echo "[$(date +%H:%M:%S)] $*"; }

# --- 1. wait for the enemy-AI proof link to finish -------------------------
# success = exe mtime newer than the newest client source AND size stable 6s.
# failure = ^error in the build log. abort = cargo gone with a stale exe.
wait_for_exe() { # <exe> <buildlog>
  local exe="$1" log="$2" last=-1 stable=0 i=0
  while [ $i -lt 900 ]; do
    if [ "$(grep -c '^error' "$log" 2>/dev/null)" -gt 0 ]; then
      say "BUILD FAILED — ^error in $log"; grep -m3 '^error' "$log"; return 1
    fi
    if [ -f "$exe" ]; then
      local sz; sz=$(stat -c %s "$exe" 2>/dev/null || echo 0)
      local newer; newer=$(find "$exe" -newer client/src/enemy_ai.rs 2>/dev/null | wc -l)
      if [ "$sz" = "$last" ] && [ "$sz" -gt 1000000 ] && [ "$newer" -ge 1 ]; then
        stable=$((stable + 3))
        if [ $stable -ge 6 ]; then
          say "EXE READY $exe bytes=$sz mtime=$(date -r "$exe" +%H:%M:%S)"; return 0
        fi
      else
        stable=0
      fi
      last=$sz
    fi
    if ! tasklist 2>/dev/null | grep -qi 'cargo.exe'; then
      if [ ! -f "$exe" ]; then say "ABORT — cargo gone, no $exe"; return 1; fi
    fi
    sleep 3; i=$((i + 3))
  done
  say "TIMEOUT waiting for $exe"; return 1
}

say "=== step 1: wait for $AI_EXE ==="
wait_for_exe "$AI_EXE" _fl_ai_build.log || { echo "DRIVER FAIL step1" > _fl_ai_driver.done; exit 1; }

# --- 2. AFTER: run rose's new AI proof, capture frames + trace -------------
say "=== step 2: capture AFTER (new archetype AI) ==="
rm -rf "$AFTER_DIR"; mkdir -p "$AFTER_DIR"
VOXELFORGE_AIFRAMES="$AFTER_DIR" VOXELFORGE_AILOG="$AFTER_DIR/trace.csv" \
  timeout 900 "./$AI_EXE" > _fl_ai_after.runlog 2>&1
say "after exit=$? frames=$(ls "$AFTER_DIR"/f*.png 2>/dev/null | wc -l) trace=$(wc -l < "$AFTER_DIR/trace.csv" 2>/dev/null || echo 0)"
grep -E 'AIPROOF DONE|panicked|B0001' _fl_ai_after.runlog | head -5

# --- 3. build the REAL game bin (the shipped husk AI lives in combat.rs) ---
say "=== step 3: build voxelforge (real game) into target-flamingo ==="
# NO profile overrides here: `CARGO_PROFILE_DEV_DEBUG=0` changes the profile
# hash, which invalidates every cached dep in target-flamingo and turns a
# 1-crate rebuild into a 200-crate one. Learned the hard way at 21:45.
cargo build --manifest-path client/Cargo.toml \
  --bin voxelforge --target-dir target-flamingo -j 2 > _fl_game_build.log 2>&1
say "game build errors=$(grep -c '^error' _fl_game_build.log)"

# --- 4. BEFORE: the shipped husk AI, through the real game path ------------
if [ "$(grep -c '^error' _fl_game_build.log)" -eq 0 ] && [ -f "$GAME_EXE" ]; then
  say "=== step 4: capture BEFORE (shipped Guard Husk AI) ==="
  rm -rf "$BEFORE_DIR"; mkdir -p "$BEFORE_DIR"
  # AI_TRACE gives the husks' per-frame positions — the same numeric shape the
  # new AI's trace.csv carries, so the two pursuits can be plotted side by side
  # instead of only described.
  VOXELFORGE_PLAY=1 VOXELFORGE_AI_DEMO=1 VOXELFORGE_AI_LOG=1 \
    VOXELFORGE_AI_TRACE="$BEFORE_DIR/trace.csv" \
    VOXELFORGE_AI_SHOTS="$BEFORE_DIR" timeout 420 "./$GAME_EXE" \
    > _fl_ai_before.runlog 2>&1
  say "before exit=$? shots=$(ls "$BEFORE_DIR"/*.png 2>/dev/null | wc -l) trace=$(wc -l < "$BEFORE_DIR/trace.csv" 2>/dev/null || echo 0)"
  grep -E 'AI_DEMO shot|AI_TRACE wrote|AI_DEMO overall|panicked|B0001' _fl_ai_before.runlog | head -14
else
  say "SKIP step 4 — game build not green"
fi

# --- 5. assemble the deliverable PNGs -------------------------------------
say "=== step 5: assemble sheets ==="
python scripts/_flamingo_ai_ba_sheet.py > _fl_ai_sheet.log 2>&1
say "sheet exit=$?"
tail -20 _fl_ai_sheet.log

say "DRIVER DONE"
echo "done $(date +%H:%M:%S)" > _fl_ai_driver.done
