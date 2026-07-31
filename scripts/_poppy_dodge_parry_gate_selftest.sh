#!/usr/bin/env bash
# Self-test for `scripts/prove_dodge_parry.sh` — proves the GATE bites.
#
# This grades the grader, not the game. `docs/LANES.md`: "A gate that can't
# fail is worse than no gate — it launders a broken build into a green report."
# `prove_playable.sh` once printed `PASS 4/4` next to a `FAIL` line for exactly
# this reason, so before the dodge/parry gate is trusted on a real run, every
# assertion in it is shown to reject a log that violates it.
#
# Method: build one synthetic log shaped like a healthy run, confirm the gate
# passes it, then mutate that log one property at a time and require the gate
# to fail AND to name the right tag. A mutation the gate lets through is a
# decorative assertion and is reported here as a failure of the gate.
#
#   bash scripts/_poppy_dodge_parry_gate_selftest.sh
set -uo pipefail

GATE=scripts/prove_dodge_parry.sh
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
GOLD="$WORK/golden.log"

# ---------------------------------------------------------------------------
# A synthetic log in the exact shape the shipping systems emit. Hand-written on
# purpose: if the gate only ever sees logs the game produced, a gate that reads
# the wrong field name passes forever without anyone noticing.
#
# Note the successful parry at frame 400 has NO matching FEEL_PARRY_CLOSE: a
# parry that lands *spends* its window early, and `watch_windows` deliberately
# does not log that as a close. Only a window that expired unused (frames
# 600–612 below) is evidence about the window's length.
# ---------------------------------------------------------------------------
cat >"$GOLD" <<'EOF'
DODGE_PROBE spawn husk #1 at (0.0,1.0,-4.0)
FEEL_HUSK_RHYTHM entity=4294967297 combo=1 pattern=straight hold=0.80 follow_up=false
FEEL_IFRAME open_frame=100 booked=10 pos=(0.00,1.60,-1.00)
FEEL_DODGE_NEGATE frame=100 frames_left=10 booked=10 dmg_denied=15.0 hp=100.0
FEEL_IFRAME_CLOSE open_frame=100 close_frame=110 frames_open=10 booked=10
FEEL_IMPACT pos=(0.00,1.00,-4.00) dir=(0.00,-1.00) weight=light hitstop=0.080 knockback=0.180 kick=0.045 target=4294967297 attacker=8589934593
FEEL_KNOCKBACK entity=4294967297 from=(0.00,-4.00) to=(0.00,-4.18) moved=0.180 impulse=0.180 slid=0.180 topup=0.000 dir=(0.00,-1.00)
FEEL_IFRAME open_frame=220 booked=10 pos=(0.00,1.60,-1.00)
FEEL_DODGE_NEGATE frame=222 frames_left=8 booked=10 dmg_denied=20.0 hp=100.0
FEEL_IFRAME_CLOSE open_frame=220 close_frame=230 frames_open=10 booked=10
FEEL_PARRY_OPEN open_frame=400 booked=12
FEEL_PARRY frame=400 open_frame=400 in_window=1 booked=12 attacker=4294967297 posture_before=30.0 dealt=25.0 posture_after=5.0 posture_final=0.0 broke=true shove=0.320 hitstop=0.120 riposte_window=1.200
FEEL_RIPOSTE frame=418 target=4294967297 mult=3.25 dmg=65.0 weight=critical hitstop=0.170 knockback=0.550 kick=0.155 hp=15.0
FEEL_IMPACT pos=(0.00,1.00,-4.00) dir=(0.00,-1.00) weight=critical hitstop=0.170 knockback=0.550 kick=0.155 target=4294967297 attacker=8589934593
FEEL_KNOCKBACK entity=4294967297 from=(0.00,-4.00) to=(0.00,-4.55) moved=0.550 impulse=0.550 slid=0.550 topup=0.000 dir=(0.00,-1.00)
FEEL_PARRY_OPEN open_frame=600 booked=12
FEEL_PARRY_CLOSE open_frame=600 close_frame=612 frames_open=12 booked=12
FEEL_PARRY_FAIL frame=640 open_frame=600 booked=12 late_by=28 dmg_taken=25.0 hp=75.0
DODGE_PROBE done t=45.0s frame=2700 husks_spawned=2 drove_dodge_on_time=3 drove_dodge_late=2 drove_parry_on_time=3 drove_parry_late=2 player_hp=62
exit=0
EOF

PASSES=0
FAILURES=0
ok()  { echo "  ✓ $1"; PASSES=$((PASSES + 1)); }
bad() { echo "  => SELFTEST FAIL: $1"; FAILURES=$((FAILURES + 1)); }

run_gate() { bash "$GATE" --log "$1" >"$1.out" 2>&1; echo $?; }

# ---- 1. the healthy log must PASS -----------------------------------------
echo "=== baseline: a healthy log must PASS ==="
rc=$(run_gate "$GOLD")
if [ "$rc" -eq 0 ] && grep -q 'PROVE_DODGE_PARRY: PASS' "$GOLD.out"; then
  ok "golden log => PASS (exit 0)"
else
  bad "the gate rejects a healthy log (exit=$rc) — it would fail a good build"
  sed 's/^/      /' "$GOLD.out"
fi

# ---- 2. every mutation must FAIL, and name its own tag ---------------------
# mutate <name> <expected FAIL tag> <sed//delete program>
mutate() {
  local name="$1" tag="$2" prog="$3"
  local f="$WORK/$name.log"
  eval "$prog" <"$GOLD" >"$f"
  if cmp -s "$GOLD" "$f"; then
    bad "$name: the mutation changed nothing (the self-test itself is broken)"
    return
  fi
  local rc; rc=$(run_gate "$f")
  if [ "$rc" -eq 0 ]; then
    bad "$name: gate PASSED a log that violates [$tag] — that assertion is decorative"
    return
  fi
  if ! grep -q "\[$tag\]" "$f.out"; then
    bad "$name: gate failed, but never named [$tag] (it failed for the wrong reason)"
    grep -a 'FAIL' "$f.out" | sed 's/^/      /' | head -4
    return
  fi
  ok "$name => FAIL [$tag]"
}

echo ""
echo "=== mutations: each must be rejected, by the right gate ==="

# G0 — process health
mutate exit_nonzero      EXIT       "sed 's/^exit=0$/exit=101/'"
mutate panic_in_log      PANIC      "sed '2i thread \"main\" panicked at src/lib.rs:1:1'"
mutate probe_cut_short   PROBE_DONE "grep -v 'DODGE_PROBE done'"

# G1 — the i-frame window is an exact frame count
mutate iframe_short      IFRAME_EXACT   "sed '0,/frames_open=10/s//frames_open=9/'"
mutate iframe_long       IFRAME_EXACT   "sed '0,/frames_open=10/s//frames_open=11/'"
mutate iframe_rebooked   IFRAME_BOOKED  "sed 's/frames_open=10 booked=10/frames_open=8 booked=8/'"
mutate iframe_one_sample IFRAME_EXACT   "grep -v 'open_frame=220'"

# G2 — those frames really ate a swing
mutate no_negate         IFRAME_NEGATE  "grep -v FEEL_DODGE_NEGATE"
mutate negate_zero_dmg   NEGATE_DMG     "sed 's/dmg_denied=15.0/dmg_denied=0.0/'"
mutate negate_closed     NEGATE_WINDOW  "sed 's/frames_left=10/frames_left=0/'"

# G3 — the window has an edge (this is the anti-"invulnerable forever" gate)
mutate never_hurt        IFRAME_EDGE    "sed 's/player_hp=62/player_hp=100/'"

# G4 — the parry window is an exact frame count
mutate parry_wide        PARRY_EXACT    "sed 's/frames_open=12/frames_open=14/'"
mutate no_parry_close    PARRY_EXACT    "grep -v FEEL_PARRY_CLOSE"
# If a future change drops the spent/expired distinction, a landed parry starts
# reporting a 1-frame close and the window length silently stops being 12.
mutate spent_logged_as_close PARRY_EXACT \
  "sed '/^FEEL_PARRY frame=400/a FEEL_PARRY_CLOSE open_frame=400 close_frame=401 frames_open=1 booked=12'"

# G5 — §2.5's posture, dealt by the parry, arithmetic intact
mutate parry_no_posture  PARRY_POSTURE  "sed 's/dealt=25.0/dealt=0.0/'"
mutate parry_bad_math    PARRY_MATH     "sed 's/posture_after=5.0/posture_after=30.0/'"
mutate parry_no_break    PARRY_BREAK    "sed 's/broke=true/broke=false/'"
mutate parry_chipped     PARRY_BREAK    "sed 's/posture_final=0.0/posture_final=5.0/'"
mutate parry_late_win    PARRY_WINDOW   "sed 's/in_window=1 /in_window=99 /'"

# G6 — a parry rides the weight table, it does not invent its own feel
mutate parry_soft_shove  PARRY_WEIGHT   "sed 's/shove=0.320/shove=0.180/'"
mutate parry_no_hitstop  PARRY_WEIGHT   "sed 's/hitstop=0.120 riposte_window/hitstop=0.000 riposte_window/'"

# G7–G9 — the riposte exists, is Critical, and is worth the read
mutate no_riposte        RIPOSTE        "grep -v FEEL_RIPOSTE"
mutate riposte_light     RIPOSTE_WEIGHT "sed 's/weight=critical hitstop=0.170 knockback=0.550 kick=0.155 hp=/weight=light hitstop=0.170 knockback=0.550 kick=0.155 hp=/'"
mutate riposte_no_freeze RIPOSTE_WEIGHT "sed 's/hitstop=0.170 knockback=0.550 kick=0.155 hp=/hitstop=0.080 knockback=0.550 kick=0.155 hp=/'"
mutate riposte_weak      RIPOSTE_REWARD "sed 's/dmg=65.0/dmg=25.0/'"
mutate riposte_low_mult  RIPOSTE_REWARD "sed 's/mult=3.25/mult=1.25/'"

# G10 — a mistimed parry is punished (anti-"the window never closes")
mutate no_parry_fail     PARRY_RISK     "grep -v FEEL_PARRY_FAIL"
mutate fail_not_late     PARRY_RISK     "sed 's/late_by=28/late_by=0/'"
mutate fail_no_punish    PARRY_RISK     "sed 's/dmg_taken=25.0/dmg_taken=20.0/'"

# G11 — a riposte must trace back to a parry on the same entity
mutate riposte_orphan    RIPOSTE_CAUSE  "sed 's/FEEL_RIPOSTE frame=418 target=4294967297/FEEL_RIPOSTE frame=418 target=1234/'"
mutate riposte_before    RIPOSTE_CAUSE  "sed 's/FEEL_RIPOSTE frame=418/FEEL_RIPOSTE frame=8/'"

# G12 — the run drove all four timings
mutate no_late_parry_run COVERAGE       "sed 's/drove_parry_late=2/drove_parry_late=0/'"
mutate no_late_dodge_run COVERAGE       "sed 's/drove_dodge_late=2/drove_dodge_late=0/'"

# G13 — the weight layer underneath is still intact in the same run
mutate weight_silent     WEIGHT_REGRESSION "grep -v FEEL_KNOCKBACK"
mutate weight_drift      WEIGHT            "sed 's/weight=light hitstop=0.080/weight=light hitstop=0.120/'"

echo ""
echo "─── SELFTEST VERDICT ───"
echo "assertions exercised: $((PASSES + FAILURES)) · ok: $PASSES · broken: $FAILURES"
if [ "$FAILURES" -gt 0 ]; then
  echo "GATE_SELFTEST: FAIL — the gate has $FAILURES assertion(s) that do not bite"
  exit 1
fi
echo "GATE_SELFTEST: PASS — every gate in prove_dodge_parry.sh rejects its own violation"
exit 0
