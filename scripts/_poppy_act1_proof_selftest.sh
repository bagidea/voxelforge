#!/usr/bin/env bash
# _poppy_act1_proof_selftest.sh — does `act1_runtime_proof.sh` actually grade?
#
# The driver cannot be trusted until it has been made to FAIL on demand. This
# stands a fake "game" in front of it that emits canned engine stdout, then
# feeds it (a) a correct log, and (b) six broken ones — the old pre-fb71e4c
# semantics, a scenario that silently ran the ordinary route, an env that was
# not honoured, an ambush that happened inside the region, and a binary with the
# marker strings stripped out.
#
# A driver that passes (a) and passes any of (b) is a driver that would sign off
# on the bug. Run this BEFORE trusting a green runtime run.
#
#   bash scripts/_poppy_act1_proof_selftest.sh
#
# Exit 0 = the driver grades correctly. Touches nothing outside _probe/act1-selftest.

set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

WORK=_probe/act1-selftest
rm -rf "$WORK"
mkdir -p "$WORK/bin/assets/audio" "$WORK/bin/assets/story"

DRIVER=scripts/act1_runtime_proof.sh
PASS=0
FAIL=0

# ---------------------------------------------------------------------------
# The canned engine stdout. `$1` picks the flavour.
# ---------------------------------------------------------------------------
make_fake_bin() {
  local flavour="$1" path="$2"
  cat > "$path" <<'FAKE_EOF'
#!/usr/bin/env bash
# Fake voxelforge.exe — emits canned engine stdout for the driver self-test.
mode="${VOXELFORGE_QUEST_CHAOS:-none}"
flavour="$FAKE_FLAVOUR"

banner_mode="$mode"
[ "$flavour" = "env_ignored" ] && banner_mode="none"

echo "STORY_LOAD ok path=assets/story/act1.json quests=5 npcs=3 dialogues=9 regions=9"
echo "QUEST_CHAOS mode=$banner_mode env=VOXELFORGE_QUEST_CHAOS"
echo "QUEST_STAGE_COMPLETE qid=q1_embers oid=o1_campfire => PASS (approach dist=1.2)"
echo "QUEST_COMPLETE id=q1_embers => PASS"
echo "QUEST_ACCEPT id=q2_voice_in_stone => PASS (from Locked)"
echo "QUEST_AREA enter=gate_square player=(32.5,6.4)"
echo "QUEST_FLAG set=entered_gate_square"
echo "QUEST_STAGE_COMPLETE qid=q2_voice_in_stone oid=o1_gate => PASS (reach_zone)"

emit_early_zone() {
  echo "QUEST_AREA enter=guard_post_east player=(49.5,8.5)"
  echo "QUEST_FLAG set=entered_guard_post_east"
}
emit_late_zone() {
  echo "QUEST_AREA enter=guard_post_east player=(49.5,8.5)"
  echo "QUEST_FLAG set=entered_guard_post_east"
}
emit_o1_east() {
  echo "QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)"
}
# Maren's scene closes q2 and hands out q3 — the moment "early" is measured from.
emit_q2_done() {
  echo "QUEST_DIALOGUE fire id=dlg_maren_gate trigger=enter_zone"
  echo "QUEST_STAGE_COMPLETE qid=q2_voice_in_stone oid=o2_listen => PASS (dialogue)"
  echo "QUEST_COMPLETE id=q2_voice_in_stone => PASS"
  echo "QUEST_ACCEPT id=q3_gatekeeper => PASS (from Locked)"
  echo "QUEST_NEXT_OPEN next=q3_gatekeeper => PASS"
}
emit_kill() {
  # The swing that did it — press side (quest_demo) and detect side (the probe
  # ordered after combat::gather_input). The triage grades the fight on these.
  echo "QUEST_DEBUG_PRESS X pressed at t=12.100 press_count=1 attempts=9 was_already_pressed=false just_pressed_now=true"
  echo "QUEST_DEBUG_ATTACK X just_pressed detect_count=1 press_calls=1 intent_light=true demo_fid=730"
  echo "QUEST_KILL qid=q3_gatekeeper oid=o3_defeat count=1/1"
  echo "QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)"
}
emit_tail() {
  echo "QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o2_observe => PASS (approach dist=3.4)"
  echo "QUEST_COMPLETE id=q3_gatekeeper => PASS"
  echo "QUEST_REWARD queue door=guard_post_door from=q3_gatekeeper"
  echo "QUEST_ACCEPT id=q4_what_walls_remember => PASS (from Locked)"
  echo "QUEST_NEXT_OPEN next=q4_what_walls_remember => PASS"
  echo "QUEST_DEBUG_PRESS R pressed at t=41.100 press_count=1 was_already_pressed=false just_pressed_now=true"
  echo "QUEST_DEBUG_BLOCK R just_pressed pt=(50.1,9.0) detect_count=1 press_calls=1 jp=[KeyR] demo_fid=2470"
  echo "QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)"
  echo "QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o2_ledger => PASS (interact)"
  echo "QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o3_offering => PASS (interact)"
  echo "QUEST_COMPLETE id=q4_what_walls_remember => PASS"
  echo "QUEST_REWARD queue door=gate_chamber_door from=q4_what_walls_remember"
  echo "QUEST_DOOR gate opened x=30..34 z=3 y=1..3"
  echo "QUEST_SIGIL glow at (32,12,5)"
  echo "QUEST_ACCEPT id=q5_sigil_that_knew_you => PASS (from Locked)"
  echo "QUEST_NEXT_OPEN next=q5_sigil_that_knew_you => PASS"
  echo "QUEST_STAGE_COMPLETE qid=q5_sigil_that_knew_you oid=o1_sigil => PASS (approach dist=2.0)"
  echo "QUEST_AREA enter=hollow_reach_intro player=(32.0,1.4)"
  echo "QUEST_STAGE_COMPLETE qid=q5_sigil_that_knew_you oid=o2_enter => PASS (reach_zone)"
  echo "QUEST_STAGE_COMPLETE qid=q5_sigil_that_knew_you oid=o3_end => PASS (listen)"
  echo "QUEST_COMPLETE id=q5_sigil_that_knew_you => PASS"
  echo "QUEST_FLAG act1_complete => PASS"
  echo "QUEST_PROOF Act 1 full chain ALL GATES PASS => PASS"
}

case "$mode:$flavour" in
  # ---- correct behaviour -------------------------------------------------
  none:good)
    emit_q2_done; emit_late_zone; emit_o1_east; emit_kill; emit_tail ;;
  zone_early:good)
    emit_early_zone; emit_q2_done; emit_o1_east; emit_kill; emit_tail ;;
  kill_early:good)
    emit_q2_done
    echo "QUEST_CHAOS ambush pt=(46.0,8.5) near=1 hp=88 t=12.0"
    emit_kill
    echo "QUEST_CHAOS ambush done at (46.0,8.5) - resuming the walk east"
    emit_late_zone; emit_o1_east; emit_tail ;;

  # ---- NC1: pre-fb71e4c kill semantics - the kill is dropped, q3 dies ----
  kill_early:old_kill)
    emit_q2_done
    echo "QUEST_CHAOS ambush timeout x=46.0 z=8.5 hp=41 => FAIL"
    echo "QUEST_FATAL phase=99" ;;

  # ---- NC2: pre-fb71e4c zone semantics - the region was spent early ------
  zone_early:old_zone)
    emit_early_zone; emit_q2_done
    echo "QUEST_WALK_GUARD_POST timeout x=49.5 z=8.5 hp=100 => FAIL"
    echo "QUEST_FATAL phase=99" ;;

  # ---- NC3/NC4: the scenario silently ran the ordinary route -------------
  kill_early:in_order)
    emit_q2_done; emit_late_zone; emit_o1_east; emit_kill
    echo "QUEST_CHAOS ambush done at (46.0,8.5) - resuming the walk east"
    emit_tail ;;
  zone_early:in_order)
    emit_q2_done; emit_late_zone; emit_o1_east; emit_kill; emit_tail ;;

  # ---- NC5: env not honoured (banner says none) --------------------------
  zone_early:env_ignored)
    emit_early_zone; emit_q2_done; emit_o1_east; emit_kill; emit_tail ;;

  # ---- NC6: the ambush happened INSIDE guard_post_east -------------------
  kill_early:inside)
    emit_q2_done
    emit_kill
    echo "QUEST_CHAOS ambush done at (49.5,8.5) - resuming the walk east"
    emit_late_zone; emit_o1_east; emit_tail ;;

  # ---- NC9: the fight is lost with X pressed but never read (ordering) ---
  kill_early:x_eaten)
    emit_q2_done
    echo "QUEST_CHAOS ambush pt=(46.0,8.5) near=1 hp=88 t=12.0"
    echo "QUEST_DEBUG_PRESS X pressed at t=12.100 press_count=1 attempts=9 was_already_pressed=false just_pressed_now=true"
    echo "QUEST_DEBUG_PRESS X pressed at t=12.400 press_count=2 attempts=27 was_already_pressed=false just_pressed_now=true"
    echo "QUEST_CHAOS ambush timeout x=46.0 z=8.5 hp=41 => FAIL"
    echo "QUEST_FATAL phase=99" ;;

  # ---- NC10: X read, but gather_input ran first (intent never lit) -------
  kill_early:x_late)
    emit_q2_done
    echo "QUEST_CHAOS ambush pt=(46.0,8.5) near=1 hp=88 t=12.0"
    echo "QUEST_DEBUG_PRESS X pressed at t=12.100 press_count=1 attempts=9 was_already_pressed=false just_pressed_now=true"
    echo "QUEST_DEBUG_ATTACK X just_pressed detect_count=1 press_calls=1 intent_light=false demo_fid=730"
    echo "QUEST_CHAOS ambush timeout x=46.0 z=8.5 hp=41 => FAIL"
    echo "QUEST_FATAL phase=99" ;;

  # ---- NC7: the region announced arrival twice (latch gone) -------------
  zone_early:twice)
    emit_early_zone; emit_q2_done
    emit_late_zone; emit_o1_east; emit_kill; emit_tail ;;

  *) emit_late_zone; emit_o1_east; emit_kill; emit_tail ;;
esac

case "$flavour" in
  old_kill|old_zone|x_eaten|x_late) exit 1 ;;
  *) exit 0 ;;
esac
FAKE_EOF
  # Bake the flavour in — the driver controls only VOXELFORGE_QUEST_CHAOS.
  sed -i "s/\$FAKE_FLAVOUR/$flavour/" "$path"
  chmod +x "$path"
}

# ---------------------------------------------------------------------------
run_case() { # run_case <name> <flavour> <scenario> <expected exit>
  local name="$1" flavour="$2" scenario="$3" want_rc="$4"
  local bin="$WORK/bin/voxelforge.exe"
  make_fake_bin "$flavour" "$bin"
  if [ "$flavour" = "stripped" ]; then
    # Remove every provenance string so the exe looks pre-fix.
    sed -i 's/QUEST_CHAOS mode=/QUEST_XXXXX mode=/g; s/(reach_zone)/(reachzone)/g; s/QUEST_REWARD queue door=/QUEST_REWARD queue dr=/g; s/QUEST_DOOR gate opened/QUEST_DOOR gate opn/g; s/QUEST_DEBUG_PRESS X pressed at t=/QUEST_DEBUG_PRESS X hit t=/g; s/QUEST_DEBUG_ATTACK X just_pressed/QUEST_DEBUG_ATTACK X jp/g' "$bin"
  fi
  local out rc
  out=$(BIN="$bin" OUTDIR="$WORK/out-$name" bash "$DRIVER" "$scenario" 2>&1)
  rc=$?
  if [ "$rc" -eq "$want_rc" ]; then
    echo "  PASS  $name — driver exit $rc (expected $want_rc)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $name — driver exit $rc, expected $want_rc"
    echo "$out" | sed 's/^/          /' | tail -25
    FAIL=$((FAIL + 1))
  fi
  printf '%s\n' "$out" > "$WORK/$name.driver.log"
}

# An exit code says the driver failed the run. It does not say the driver
# blamed the right thing — a triage that names the wrong key exits 1 just as
# confidently as one that names the right one. Grade the words too.
assert_says() { # assert_says <case> <must contain>
  local name="$1" pat="$2"
  if grep -qF -- "$pat" "$WORK/$name.driver.log"; then
    echo "  PASS  $name — triage says: $pat"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $name — triage never said: $pat"
    sed -n '/INPUT ORDERING TRIAGE/,/INPUT_TRACE/p' "$WORK/$name.driver.log" | sed 's/^/          /'
    FAIL=$((FAIL + 1))
  fi
}

assert_silent() { # assert_silent <case> <must NOT contain>
  local name="$1" pat="$2"
  if grep -qF -- "$pat" "$WORK/$name.driver.log"; then
    echo "  FAIL  $name — triage wrongly said: $pat"
    sed -n '/INPUT ORDERING TRIAGE/,/INPUT_TRACE/p' "$WORK/$name.driver.log" | sed 's/^/          /'
    FAIL=$((FAIL + 1))
  else
    echo "  PASS  $name — triage did not say: $pat"
    PASS=$((PASS + 1))
  fi
}

echo "=== driver self-test: does act1_runtime_proof.sh grade? ==="
echo ""
echo "-- positive controls (must PASS, exit 0) --"
run_case pos-none        good        none        0
run_case pos-zone-early  good        zone_early  0
run_case pos-kill-early  good        kill_early  0

echo ""
echo "-- negative controls (must FAIL, exit 1) --"
run_case nc1-old-kill-semantics   old_kill    kill_early  1
run_case nc2-old-zone-semantics   old_zone    zone_early  1
run_case nc3-kill-ran-in-order    in_order    kill_early  1
run_case nc4-zone-ran-in-order    in_order    zone_early  1
run_case nc5-env-not-honoured     env_ignored zone_early  1
run_case nc6-ambush-inside-region inside      kill_early  1
run_case nc7-zone-latched-twice   twice       zone_early  1

run_case nc9-x-press-never-read   x_eaten     kill_early  1
run_case nc10-x-read-intent-dark  x_late      kill_early  1

echo ""
echo "-- provenance gate (must refuse, exit 2) --"
run_case nc8-binary-predates-fix  stripped    kill_early  2

echo ""
echo "-- triage names the key the failing stage actually used (rule 2) --"
# NC1 dies in the ambush without ever typing an R. The old triage read R's
# counter, found 0, and announced a phase-4 walk failure — a cause invented
# from a key the stage never touched.
assert_says   nc1-old-kill-semantics "failing stage: ambush fight (key X)"
assert_silent nc1-old-kill-semantics "the demo never pressed R"
assert_says   nc1-old-kill-semantics "the demo never swung"
# Both ledgers print, always — the un-blamed key's counts are still evidence,
# and printing them is what proves the triage read it before ruling it out.
assert_says   nc1-old-kill-semantics "R (build)  emitted by quest_demo : 0"
assert_says   nc7-zone-latched-twice "X (attack) emitted by quest_demo : 1"
assert_silent nc1-old-kill-semantics "unbound variable"
# NC9: X went out, nothing read it → ordering, not approach.
assert_says   nc9-x-press-never-read "VERDICT: ORDERING — 2 X press(es) never reached the"
assert_silent nc9-x-press-never-read "the demo never swung"
# NC10: the counts balance, so only intent_light exposes the ordering bug.
assert_says   nc10-x-read-intent-dark "every detected X left intent_light=false"
# A run that got as far as the build stand is still graded on R.
assert_says   nc7-zone-latched-twice "failing stage: phase 4 build (key R)"
# NC2 dies in the walk before either key is typed. Neither ledger explains it,
# so the triage has to say so and stop — inventing a cause here is the failure
# mode rule 2 exists for.
assert_says   nc2-old-zone-semantics "failing stage: NOT IDENTIFIABLE"
assert_silent nc2-old-zone-semantics "VERDICT:"

echo ""
echo "=== SELFTEST: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] || exit 1
echo "The driver passes correct logs and refuses every broken one."
