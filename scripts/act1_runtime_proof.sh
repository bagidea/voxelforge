#!/usr/bin/env bash
# act1_runtime_proof.sh — runtime proof that the Act-1 loop walks q1 -> q5 in a
# REAL game process, including the two out-of-order routes that used to kill it.
#
# This is not a logic test. `scripts/act1_loop_audit.py` checks the story data
# against the engine statically and `client/src/quest_rules.rs` /
# `client/src/quest_chaos.rs` run headless; neither one starts the game. This
# does, three times:
#
#   none        the ordinary in-order route (the regression baseline)
#   zone_early  stand in guard_post_east BEFORE Maren hands out q3, then finish it
#   kill_early  put Garren down while o1_east is still owed, then finish it
#
# ---------------------------------------------------------------------------
# THE RULE THIS SCRIPT IS BUILT AROUND
#
# Every graded line is `println!`-ed by the engine (client/src/quest.rs). This
# script grades `$RAW` and NOTHING ELSE writes to `$RAW` — it is created by the
# `>` redirect on the game process and never appended to. The envelope, the
# grade and the triage all go to stdout and to `$LOG`, which is never graded.
# A harness that prints its own PASS lines proves the harness runs, not the game.
# ---------------------------------------------------------------------------
#
# Usage:
#   bash scripts/act1_runtime_proof.sh                     # all three scenarios
#   BIN=target-quest/debug/voxelforge.exe bash scripts/act1_runtime_proof.sh
#   bash scripts/act1_runtime_proof.sh kill_early          # just one
#
# Exit: 0 all scenarios PASS · 1 a scenario FAILED · 2 the binary is not ready.
#
# No `timeout` wrapper: the game controls its own exit via AppExit, and
# `timeout` on Windows Git Bash maps to the wrong binary and returns 127.

set -uo pipefail

BIN=${BIN:-./target-quest/debug/voxelforge.exe}
OUTDIR=${OUTDIR:-_quest_proof/act1-runtime}
SCENARIOS=("$@")
if [ ${#SCENARIOS[@]} -eq 0 ]; then
  SCENARIOS=(none zone_early kill_early)
fi

mkdir -p "$OUTDIR"

# ===========================================================================
# Gate 0 — is this binary even the one that carries the fix?
# ===========================================================================
# A commit clock never dates a binary. Scan the exe for strings that exist ONLY
# in the patched source; if they are missing, the build has not landed and every
# grade below would be a lie about an old exe.
PROVENANCE_STRINGS=(
  'QUEST_CHAOS mode='                       # quest_demo banner (quest_chaos lane)
  '=> PASS (reach_zone)'                    # check_area_triggers, presence-based
  'QUEST_REWARD queue door='                # complete_quest queues world rewards
  'QUEST_DOOR gate opened'                  # apply_world_rewards
)

echo "=== BINARY PROVENANCE ==="
echo "binary: $BIN"
if [ ! -f "$BIN" ]; then
  echo "  NOT_READY — $BIN does not exist. Build has not landed."
  exit 2
fi
echo "  mtime: $(stat -c '%y' "$BIN" 2>/dev/null || stat -f '%Sm' "$BIN" 2>/dev/null || echo unknown)"
echo "  bytes: $(stat -c '%s' "$BIN" 2>/dev/null || stat -f '%z' "$BIN" 2>/dev/null || echo unknown)"
missing=0
for s in "${PROVENANCE_STRINGS[@]}"; do
  if grep -aqF "$s" "$BIN"; then
    echo "  ok      contains: $s"
  else
    echo "  MISSING           : $s"
    missing=$((missing + 1))
  fi
done
if [ "$missing" -gt 0 ]; then
  echo ""
  echo "NOT_READY — $missing marker string(s) absent from the binary."
  echo "This exe predates the quest fix. Rebuild before grading; do not report"
  echo "these scenarios against it."
  exit 2
fi
echo ""

# Bevy resolves assets relative to the exe.
exe_dir="$(dirname "$BIN")"
for d in audio story; do
  if [ ! -d "$exe_dir/assets/$d" ] && [ -d "assets/$d" ]; then
    mkdir -p "$exe_dir/assets"
    cp -r "assets/$d" "$exe_dir/assets/$d" 2>/dev/null || true
  fi
done

# ===========================================================================
# Grading helpers — all read $RAW, which only the game wrote.
# ===========================================================================
RAW=""    # set per scenario
FAILED=0  # per-scenario failure counter

# First 1-based line number matching a fixed string, or empty.
lineno() { grep -naF -m1 -- "$1" "$RAW" | cut -d: -f1; }
count()  { grep -acF -- "$1" "$RAW"; }

want() { # want <label> <fixed string that must appear>
  if grep -qaF -- "$2" "$RAW"; then
    echo "  PASS  $1"
  else
    echo "  FAIL  $1"
    echo "          expected engine line: $2"
    FAILED=$((FAILED + 1))
  fi
}

want_absent() { # want_absent <label> <fixed string that must NOT appear>
  if grep -qaF -- "$2" "$RAW"; then
    echo "  FAIL  $1"
    echo "          engine printed: $(grep -naF -m1 -- "$2" "$RAW")"
    FAILED=$((FAILED + 1))
  else
    echo "  PASS  $1"
  fi
}

want_before() { # want_before <label> <earlier> <later>
  local a b
  a=$(lineno "$2"); b=$(lineno "$3")
  if [ -z "$a" ] || [ -z "$b" ]; then
    echo "  FAIL  $1"
    [ -z "$a" ] && echo "          missing: $2"
    [ -z "$b" ] && echo "          missing: $3"
    FAILED=$((FAILED + 1))
  elif [ "$a" -lt "$b" ]; then
    echo "  PASS  $1  (line $a before line $b)"
  else
    echo "  FAIL  $1  (line $a NOT before line $b)"
    echo "          earlier: $2"
    echo "          later  : $3"
    FAILED=$((FAILED + 1))
  fi
}

want_count() { # want_count <label> <fixed string> <n>
  local n
  n=$(count "$2")
  if [ "$n" -eq "$3" ]; then
    echo "  PASS  $1  (x$n)"
  else
    echo "  FAIL  $1  (found $n, expected $3)"
    echo "          line: $2"
    FAILED=$((FAILED + 1))
  fi
}

# ---------------------------------------------------------------------------
# Rule 2 triage: when something the demo TYPED did not land, say whether the
# flag survived the frame before anyone blames range or physics.
#
# quest.rs prints INPUT_TRACE before/after the handler and a press/detect
# counter pair. If press_calls > detect_calls the flag was eaten between
# quest_demo and check_block_place_triggers — a system-ordering bug. If they
# match, the press arrived and the objective test (distance / quest state) is
# what refused it.
# ---------------------------------------------------------------------------
input_triage() {
  echo ""
  echo "  --- INPUT ORDERING TRIAGE (rule 2: ordering before range/physics) ---"
  local pressed detected
  pressed=$(grep -ac 'QUEST_DEBUG_PRESS R pressed' "$RAW")
  detected=$(grep -ac 'QUEST_DEBUG_BLOCK R just_pressed' "$RAW")
  echo "    R presses emitted by quest_demo   : $pressed"
  echo "    R presses seen by the handler     : $detected"
  if [ "$pressed" -eq 0 ]; then
    echo "    VERDICT: the demo never pressed R — it never reached the build stand."
    echo "             This is a WALK failure, not an input one. Look at the"
    echo "             QUEST_DEBUG_P4_WALK lines below."
  elif [ "$detected" -lt "$pressed" ]; then
    echo "    VERDICT: ORDERING — $((pressed - detected)) press(es) never reached the"
    echo "             handler. Fix the schedule before touching reach/physics."
  else
    echo "    VERDICT: the flag SURVIVED the frame ($detected/$pressed seen)."
    echo "             Ordering is not the cause; the objective test refused it."
  fi
  echo "    INPUT_TRACE (first 4, last 4):"
  grep -a 'INPUT_TRACE' "$RAW" | head -4 | sed 's/^/      /'
  grep -a 'INPUT_TRACE' "$RAW" | tail -4 | sed 's/^/      /'
  echo "    Handler's own reason:"
  grep -aE 'QUEST_DEBUG_BLOCK (no place_block|too far|NO place_block|R already claimed)' "$RAW" \
    | head -6 | sed 's/^/      /'
  grep -a 'QUEST_DEBUG_P4_WALK' "$RAW" | tail -4 | sed 's/^/      /'
}

# ===========================================================================
# The chain every scenario has to finish, whatever route it took there.
# All of these come out of complete_quest / check_*_triggers / apply_world_rewards.
# ===========================================================================
grade_full_chain() {
  want "story data loaded"                 'STORY_LOAD ok'
  want "q1_embers completed"               'QUEST_COMPLETE id=q1_embers => PASS'
  want "q2_voice_in_stone completed"       'QUEST_COMPLETE id=q2_voice_in_stone => PASS'
  want "q3_gatekeeper accepted"            'QUEST_ACCEPT id=q3_gatekeeper'
  want "q3 o1_east scored (reach_zone)"    'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)'
  want "q3 o3_defeat scored (defeat)"      'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)'
  want "q3_gatekeeper completed"           'QUEST_COMPLETE id=q3_gatekeeper => PASS'
  want "q4 opened behind q3"               'QUEST_NEXT_OPEN next=q4_what_walls_remember => PASS'
  want "q4 o1_build scored (place_block)"  'QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)'
  want "q4_what_walls_remember completed"  'QUEST_COMPLETE id=q4_what_walls_remember => PASS'
  want "q4 queued the gate door"           'QUEST_REWARD queue door=gate_chamber_door'
  want "the gate actually opened"          'QUEST_DOOR gate opened'
  want "the sigil actually lit"            'QUEST_SIGIL glow'
  want "q5 o2_enter scored (reach_zone)"   'QUEST_STAGE_COMPLETE qid=q5_sigil_that_knew_you oid=o2_enter => PASS (reach_zone)'
  want "q5_sigil_that_knew_you completed"  'QUEST_COMPLETE id=q5_sigil_that_knew_you => PASS'
  want "act1_complete flag set"            'QUEST_FLAG act1_complete => PASS'
  want "engine's own end-of-act verdict"   'QUEST_PROOF Act 1 full chain ALL GATES PASS => PASS'
  want_absent "no FATAL"                   'QUEST_FATAL'
}

# ===========================================================================
# Per-scenario extra contract — the out-of-order claim itself.
# ===========================================================================
grade_scenario() {
  case "$1" in
    none)
      want "ran as the ordinary route"     'QUEST_CHAOS mode=none'
      # The baseline walks in order: the region is credited before the kill.
      want_before "in-order: o1_east scored before o3_defeat" \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)' \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)'
      ;;

    zone_early)
      want "ran as zone_early"             'QUEST_CHAOS mode=zone_early'
      # THE CLAIM: the player stood in guard_post_east before q3 existed...
      want_before "stood in guard_post_east BEFORE q3 was handed out" \
        'QUEST_AREA enter=guard_post_east' \
        'QUEST_ACCEPT id=q3_gatekeeper'
      want_before "...and the zone flag was set on that early visit" \
        'QUEST_FLAG set=entered_guard_post_east' \
        'QUEST_ACCEPT id=q3_gatekeeper'
      # ...and the region announced arrival exactly ONCE. This is the whole
      # point: the old code hung the objective test off that one-shot arrival,
      # so a second visit had nothing to fire. o1_east below is therefore scored
      # with no arrival event of its own — presence, not entry.
      want_count "guard_post_east announced arrival only once" \
        'QUEST_AREA enter=guard_post_east' 1
      want_before "o1_east scored AFTER q3 was handed out, on the return visit" \
        'QUEST_ACCEPT id=q3_gatekeeper' \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)'
      ;;

    kill_early)
      want "ran as kill_early"             'QUEST_CHAOS mode=kill_early'
      # THE CLAIM: the kill was credited while o1_east was still owed.
      want_before "o3_defeat credited BEFORE o1_east" \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)' \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)'
      # ...and it happened outside the region, not just before the log line.
      want_before "the kill landed before the player ever entered guard_post_east" \
        'QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)' \
        'QUEST_AREA enter=guard_post_east'
      want "the ambush reported its stand"  'QUEST_CHAOS ambush done at'
      # The engine prints the stand it died at; guard_post_east starts at x=48.
      local stand
      stand=$(grep -aoE 'QUEST_CHAOS ambush done at \(-?[0-9.]+' "$RAW" | head -1 | grep -oE '\(-?[0-9.]+' | tr -d '(')
      if [ -n "$stand" ] && awk "BEGIN{exit !($stand < 48)}"; then
        echo "  PASS  stand x=$stand is west of guard_post_east.x0=48"
      else
        echo "  FAIL  stand x=${stand:-<unparsed>} is not west of guard_post_east.x0=48"
        FAILED=$((FAILED + 1))
      fi
      ;;
  esac
}

# ===========================================================================
# Run
# ===========================================================================
OVERALL=0
SUMMARY=()

for mode in "${SCENARIOS[@]}"; do
  RAW="$OUTDIR/$mode.raw.log"      # game stdout ONLY — never appended to
  LOG="$OUTDIR/$mode.grade.log"    # envelope + grade — never graded
  FAILED=0

  {
    echo "PROOF_START $(date '+%Y-%m-%d %H:%M:%S')"
    echo "SCENARIO=$mode"
    echo "BINARY=$BIN"
  } > "$LOG"

  echo "=== RUN $mode ==="
  if [ "$mode" = "none" ]; then
    VOXELFORGE_QUEST_CHAOS= "$BIN" --quest-demo > "$RAW" 2>&1
  else
    VOXELFORGE_QUEST_CHAOS="$mode" "$BIN" --quest-demo > "$RAW" 2>&1
  fi
  rc=$?
  echo "EXIT_CODE=$rc" >> "$LOG"
  echo "  process exit: $rc   log: $RAW ($(wc -l < "$RAW" | tr -d ' ') lines)"
  echo ""

  grade_full_chain
  grade_scenario "$mode"

  if [ "$rc" -eq 0 ]; then
    echo "  PASS  process exit 0"
  else
    echo "  FAIL  process exit $rc (expected 0)"
    FAILED=$((FAILED + 1))
  fi

  if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "  --- engine FAIL / FATAL lines ---"
    grep -naE '=> FAIL|QUEST_FATAL|timeout' "$RAW" | head -12 | sed 's/^/    /'
    input_triage
    SUMMARY+=("$mode: FAIL ($FAILED checks)")
    OVERALL=1
  else
    SUMMARY+=("$mode: PASS")
  fi

  { echo "CHECKS_FAILED=$FAILED"; echo "PROOF_END $(date '+%Y-%m-%d %H:%M:%S')"; } >> "$LOG"
  echo ""
done

echo "=== SUMMARY ==="
for s in "${SUMMARY[@]}"; do echo "  $s"; done
echo ""
if [ "$OVERALL" -eq 0 ]; then
  echo "ACT1_RUNTIME_PROOF: PASS — every gate above was read out of the game's own stdout."
else
  echo "ACT1_RUNTIME_PROOF: FAIL — see the failing checks above."
fi
exit "$OVERALL"
