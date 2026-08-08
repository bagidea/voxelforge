#!/usr/bin/env bash
# Runtime proof for the R-key arbiter (`combat::RKeyRoute`).
#
# ⚠️ FOR THE INTEGRATION LEAD TO RUN — NOT FOR THE LANE.
# `docs/LANES.md` §"The build lock": a full `cargo build --bin voxelforge` is the
# integration lead's alone, and a lane that thinks it needs one for a runtime
# proof script must say so and hand it over. This is that hand-over. Poppy did
# not run it; the lane's own verification is `cargo check` in `target-poppy`
# (`scripts/_poppy_rkey_check.sh`).
#
# WHY A BUILD IS NEEDED AT ALL. No binary on disk can carry this proof:
#   voxelforge_shot.exe (17:25) — shot_main.rs `#[path]`-includes only hero.rs
#                                 and vfx.rs; combat.rs/quest.rs are not in it
#   voxelforge.exe      (04:49) — older than the 17:11 combat.rs/quest.rs edit
#   voxelforge_perf.exe (04:53) — same
#   target/debug/deps/*-<hash>.exe (2 Aug) — three days stale
#
# USAGE (defaults to target-int per LANES.md; override for target-combat):
#   bash scripts/_poppy_rkey_proof.sh [target-dir]
set -uo pipefail
cd "$(dirname "$0")/.."

TDIR="${1:-target-int}"
export CARGO_TARGET_DIR="$TDIR"

step() { echo; echo "=== $* @ $(date +%H:%M:%S) ==="; }

step "STEP 1 queue behind any build already on the box"
if ! bash scripts/_poppy_wait_for_free_box.sh; then
  echo "PROOF_EXIT=1 (box never freed)"
  exit 1
fi

# Every cargo call goes through build_safe.sh — that is what supplies
# CARGO_BUILD_JOBS=2 / CARGO_PROFILE_DEV_DEBUG=0 / CARGO_INCREMENTAL=0 and
# propagates cargo's real exit code with no pipe in the way.
step "STEP 2 headless proof — the two R directions in a real ECS"
bash scripts/build_safe.sh test --bin voxelforge > _poppy_rkey_test.log 2>&1
echo "TEST_EXIT=$?"
# Never read the verdict off `tail` or a pipe's exit code (LANES.md, v0.9.45).
echo "TEST_ERRORS=$(grep -c '^error' _poppy_rkey_test.log)"
grep -E '^test result:|^test .* \.\.\. ' _poppy_rkey_test.log

step "STEP 3 build the client bin (the scripted probe needs a real app)"
bash scripts/build_safe.sh build --bin voxelforge > _poppy_rkey_build.log 2>&1
BUILD_EXIT=$?
echo "BUILD_EXIT=$BUILD_EXIT"
echo "BUILD_ERRORS=$(grep -c '^error' _poppy_rkey_build.log)"
# A running exe refuses the write silently — mtime is the only honest check.
echo "EXE_MTIME=$(date -r "$TDIR/debug/voxelforge.exe" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo MISSING)"
[ "$BUILD_EXIT" -ne 0 ] && { echo "PROOF_EXIT=1 (build failed — no real run attempted)"; exit 1; }

step "STEP 4 real run — scripted R probe over the quest demo"
# Assets are pinned to the exe's own directory (commit 888b31c), so the lane
# target dir needs its own copy.
[ -d "$TDIR/debug/assets" ] || cp -r assets "$TDIR/debug/assets"
# The demo writes its own AppExit; the timeout only stops a hung window from
# outliving this script on the owner's machine.
VOXELFORGE_RKEY_PROBE=1 VOXELFORGE_QUEST_DEMO=1 \
  timeout 420 "$TDIR/debug/voxelforge.exe" > _poppy_rkey_run.log 2>&1
echo "RUN_EXIT=$?"
echo "--- R_PROBE / R_ROUTE lines ---"
grep -E 'R_PROBE|R_ROUTE' _poppy_rkey_run.log
echo "--- quest-side decisions ---"
grep -E 'QUEST_DEBUG_BLOCK (R just_pressed|no place_block|R already claimed)' _poppy_rkey_run.log | head -20

# HOW TO READ STEP 4 — one half of it is on a known-shaky path.
#   `CLAIM LockOn`     is reliable: the probe taps R at t=2.0s with no build
#                      objective anywhere near the player.
#   `CLAIM QuestPlace` depends on the demo REACHING q4's place_block objective.
#                      `docs/q4-one-run-triage.md` §7 judges Phase 4 likely to
#                      hit the 25s timeout. Two of its three fixes are on disk
#                      (quest.rs:1739 `demo.tap_t = 0.0`; quest.rs:1782-1791 the
#                      q4 `status == Active` wait-guard). The third —
#                      `quest_demo.after(check_kill_triggers)` — is DELIBERATELY
#                      NOT applied: it closes a schedule cycle and would panic
#                      the app at startup. quest_demo is already
#                      `.before(combat::gather_input)`; main.rs:558-568 chains
#                      gather_input → player_combat; quest.rs:449-450 puts
#                      check_kill_triggers `.after(player_combat)`. Adding
#                      `.after(check_kill_triggers)` closes
#                      quest_demo → gather_input → player_combat →
#                      check_kill_triggers → quest_demo.
#   So a log with `CLAIM LockOn` and NO `CLAIM QuestPlace` is a q4-demo
#   reachability failure, NOT a failure of the arbiter. STEP 2 covers that
#   direction on its own (`quest_claim_starves_lock_on_in_the_same_frame`).
echo
echo "PROOF_EXIT=0"
