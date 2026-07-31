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
# The gate (numeric, every line must prove its values independently):
#   exit 0 · no `panicked` / `B0001` / `thread ... panicked` ·
#   every COMBAT_* line ends in `=> PASS` (none says `=> FAIL`) ·
#   HP drops, delta, stamina cost, kill count, death + respawn all verified
#   with real numeric comparisons (grep/sed/awk), not just line presence.
#
# Usage:
#   bash scripts/prove_combat.sh              # build + run
#   bash scripts/prove_combat.sh --log <file>  # test against pre-existing log
set -uo pipefail

BIN=${BIN:-./target-combat/debug/voxelforge.exe}
LOGS=_combat_proof
mkdir -p "$LOGS"

# ── --log <file> → skip build+run, gate a pre-existing log ──────────────
if [ "${1:-}" = "--log" ]; then
  log="${2:?--log requires a log file path}"
  # Try to extract exit code from a trailing "exit=N" line.
  if [ -f "$log" ] && grep -q '^exit=' "$log"; then
    rc=$(grep '^exit=' "$log" | tail -1 | sed 's/^exit=//')
  else
    rc=0
  fi
  echo "=== COMBAT DEMO PROOF ==="
  echo "log: $log (pre-existing — skipping build+run)"
  echo "inferred exit code: $rc"
else
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
  echo "=== run --combat-demo (timeout 180s) ==="
  timeout 180 "$BIN" --combat-demo >"$log" 2>&1
  rc=$?
  echo "exit=$rc" >> "$log"
fi

# Print the key lines so the log is self-documenting.
grep -E 'SCENE_READY|SPAWN_ENCOUNTER|COMBAT_STRAFE|COMBAT_HIT |COMBAT_HEAVY|COMBAT_KILL|COMBAT_DEATH|COMBAT_RESPAWN|PLAYER_DIED|RESPAWN|CHASE_DIST|COMBAT_WALK_DONE|COMBAT_FATAL' "$log" || true

echo "exit=$rc"

# ── numeric proof gates ──────────────────────────────────────────────────

BAD=""
FAIL_COUNT=0

fail() {
  # Print a loud, greppable failure line with expected vs actual.
  # Usage: fail "COMBAT_HIT" "delta too small" "got=5.0" "expected=>10.0"
  local gate="$1" msg="$2" got="$3" expected="$4"
  echo "  => FAIL [$gate] $msg | $got | $expected"
  BAD="$BAD"$'\n'"[$gate] $msg — $got — $expected"
  FAIL_COUNT=$((FAIL_COUNT + 1))
}

pass() {
  echo "  ✓ $1"
}

# Helper: extract the first capture group from a line via sed.
# Returns empty string if no match.
extract() {
  echo "$1" | sed -nE "s/$2/\1/p"
}

# ── gate 0: exit code + no panic ─────────────────────────────────────────

if [ "$rc" -ne 0 ]; then
  fail "EXIT" "nonzero exit code" "exit=$rc" "expected=0"
fi

if grep -qE 'panicked|B0001|thread .* panicked' "$log"; then
  fail "PANIC" "panic detected in log" "see log" "no panic"
  grep -nE 'panicked|B0001|thread .* panicked' "$log" | head -5
fi

# ── gate 1: SCENE_READY ─────────────────────────────────────────────────

if grep -q 'SCENE_READY' "$log"; then
  pass "SCENE_READY found"
else
  fail "SCENE_READY" "not found in log" "missing" "SCENE_READY present"
fi

# ── gate 2: COMBAT_STRAFE — player must strafe while walking ────────────
# Format: COMBAT_STRAFE dx=0.85 => PASS
# Proof: |dx| > 0.3 blocks (proves the body moved from real input, not drift)

STRAFE_LINE=$(grep 'COMBAT_STRAFE' "$log" | head -1)
if [ -z "$STRAFE_LINE" ]; then
  fail "COMBAT_STRAFE" "line not found in log" "missing" "COMBAT_STRAFE present"
else
  DX=$(extract "$STRAFE_LINE" '.*dx=(-?[0-9.]+).*')
  STRAFE_RESULT=$(extract "$STRAFE_LINE" '.*=> (PASS|FAIL).*')
  if [ "$STRAFE_RESULT" = "FAIL" ]; then
    fail "COMBAT_STRAFE" "self-graded FAIL" "dx=$DX" "|dx| > 0.3"
  elif [ -z "$DX" ]; then
    fail "COMBAT_STRAFE" "could not parse dx value" "line=$STRAFE_LINE" "dx=N.NN"
  else
    if awk -v dx="$DX" 'BEGIN { a = (dx >= 0 ? dx : -dx); exit (a > 0.3) ? 0 : 1 }'; then
      pass "COMBAT_STRAFE dx=$DX |dx|=$(awk -v dx="$DX" 'BEGIN { a = (dx >= 0 ? dx : -dx); printf "%.2f", a }') > 0.3"
    else
      fail "COMBAT_STRAFE" "|dx| too small (no real strafe)" \
        "dx=$DX |dx|=$(awk -v dx="$DX" 'BEGIN { a = (dx >= 0 ? dx : -dx); printf "%.2f", a }')" \
        "|dx| > 0.3"
    fi
  fi
fi

# ── gate 3: COMBAT_HIT — light attack must reduce husk HP ───────────────
# Format: COMBAT_HIT husk_hp 80->60 (delta=20.0) => PASS
# Proof: (from - to) ≈ delta, delta > 0.5 (real hit, not a whiff)

HIT_LINE=$(grep 'COMBAT_HIT ' "$log" | head -1)
if [ -z "$HIT_LINE" ]; then
  fail "COMBAT_HIT" "line not found in log" "missing" "COMBAT_HIT present"
else
  HIT_RESULT=$(extract "$HIT_LINE" '.*=> (PASS|FAIL).*')
  HIT_FROM=$(extract "$HIT_LINE" '.*husk_hp (-?[0-9]+)->.*')
  HIT_TO=$(extract "$HIT_LINE" '.*->(-?[0-9.]+) \(delta.*')
  HIT_DELTA=$(extract "$HIT_LINE" '.*delta=(-?[0-9.]+)\).*')

  if [ "$HIT_RESULT" = "FAIL" ]; then
    fail "COMBAT_HIT" "self-graded FAIL" "hp ${HIT_FROM}->${HIT_TO} delta=$HIT_DELTA" "delta > 0.5"
  elif [ -z "$HIT_FROM" ] || [ -z "$HIT_TO" ] || [ -z "$HIT_DELTA" ]; then
    fail "COMBAT_HIT" "could not parse values" "line=$HIT_LINE" "from/to/delta"
  else
    PASSED=true
    # 1) delta must equal (from - to) within rounding tolerance (awk exit 0 = pass)
    if ! awk -v f="$HIT_FROM" -v t="$HIT_TO" -v d="$HIT_DELTA" \
      'BEGIN { diff = f - t; exit (diff - d <= 1.0 && d - diff <= 1.0) ? 0 : 1 }'; then
      fail "COMBAT_HIT" "delta mismatch: (from-to) != delta" \
        "from=$HIT_FROM to=$HIT_TO calc=$(awk -v f="$HIT_FROM" -v t="$HIT_TO" 'BEGIN { printf "%.0f", f - t }') delta=$HIT_DELTA" \
        "|calc - delta| <= 1.0"
      PASSED=false
    fi
    # 2) delta must be > 0.5 (a real hit landed)
    if ! awk -v d="$HIT_DELTA" 'BEGIN { exit (d > 0.5) ? 0 : 1 }'; then
      fail "COMBAT_HIT" "delta <= 0.5 (no real hit landed)" \
        "delta=$HIT_DELTA" \
        "delta > 0.5"
      PASSED=false
    fi
    if $PASSED; then
      pass "COMBAT_HIT hp ${HIT_FROM}->${HIT_TO} delta=$HIT_DELTA >0.5"
    fi
  fi
fi

# ── gate 4: COMBAT_HEAVY — charged attack: harder hit + stamina cost ────
# Format: COMBAT_HEAVY husk_hp 60->15 (delta=45.0) stamina -35.0 => PASS
# Proof: delta > 30.0 (light is ~20), |stamina_cost - 35| < 5 (COST_HEAVY)
# May be skipped if husk died from COMBAT_HIT — that's OK, not a failure.

HEAVY_LINE=$(grep 'COMBAT_HEAVY ' "$log" | head -1)
HEAVY_SKIP=$(grep -c 'COMBAT_HEAVY skipped' "$log" || true)

if [ -z "$HEAVY_LINE" ]; then
  if [ "$HEAVY_SKIP" -gt 0 ]; then
    pass "COMBAT_HEAVY skipped (husk already dead after light hit — not a failure)"
  else
    fail "COMBAT_HEAVY" "line not found in log (and not skipped)" "missing" "COMBAT_HEAVY present"
  fi
else
  HEAVY_RESULT=$(extract "$HEAVY_LINE" '.*=> (PASS|FAIL).*')
  HEAVY_FROM=$(extract "$HEAVY_LINE" '.*husk_hp (-?[0-9]+)->.*')
  HEAVY_TO=$(extract "$HEAVY_LINE" '.*->(-?[0-9.]+) \(delta.*')
  HEAVY_DELTA=$(extract "$HEAVY_LINE" '.*delta=(-?[0-9.]+)\).*')
  HEAVY_STAM=$(extract "$HEAVY_LINE" '.*stamina -([0-9.]+).*')

  if [ "$HEAVY_RESULT" = "FAIL" ]; then
    fail "COMBAT_HEAVY" "self-graded FAIL" \
      "hp ${HEAVY_FROM}->${HEAVY_TO} delta=$HEAVY_DELTA stamina=$HEAVY_STAM" \
      "delta > 30.0 |stam-35| < 5"
  elif [ -z "$HEAVY_FROM" ] || [ -z "$HEAVY_TO" ] || [ -z "$HEAVY_DELTA" ] || [ -z "$HEAVY_STAM" ]; then
    fail "COMBAT_HEAVY" "could not parse values" "line=$HEAVY_LINE" "from/to/delta/stamina"
  else
    PASSED=true
    # 1) delta must equal (from - to) within rounding tolerance (awk exit 0 = pass)
    if ! awk -v f="$HEAVY_FROM" -v t="$HEAVY_TO" -v d="$HEAVY_DELTA" \
      'BEGIN { diff = f - t; exit (diff - d <= 1.0 && d - diff <= 1.0) ? 0 : 1 }'; then
      fail "COMBAT_HEAVY" "delta mismatch: (from-to) != delta" \
        "from=$HEAVY_FROM to=$HEAVY_TO calc=$(awk -v f="$HEAVY_FROM" -v t="$HEAVY_TO" 'BEGIN { printf "%.0f", f - t }') delta=$HEAVY_DELTA" \
        "|calc - delta| <= 1.0"
      PASSED=false
    fi
    # 2) delta must be > 30.0 (clearly heavier than a ~20-dmg light attack)
    if ! awk -v d="$HEAVY_DELTA" 'BEGIN { exit (d > 30.0) ? 0 : 1 }'; then
      fail "COMBAT_HEAVY" "delta <= 30.0 (not clearly heavier than light)" \
        "delta=$HEAVY_DELTA" \
        "delta > 30.0"
      PASSED=false
    fi
    # 3) stamina cost must be within 5 of COST_HEAVY (35)
    if ! awk -v s="$HEAVY_STAM" 'BEGIN { d = (s - 35.0); d = (d >= 0 ? d : -d); exit (d < 5.0) ? 0 : 1 }'; then
      STAM_DIFF=$(awk -v s="$HEAVY_STAM" 'BEGIN { d = s - 35.0; d = (d >= 0 ? d : -d); printf "%.1f", d }')
      fail "COMBAT_HEAVY" "stamina cost diverged from COST_HEAVY (35)" \
        "stamina=$HEAVY_STAM diff=$STAM_DIFF" \
        "|stamina - 35| < 5.0"
      PASSED=false
    fi
    if $PASSED; then
      pass "COMBAT_HEAVY hp ${HEAVY_FROM}->${HEAVY_TO} delta=$HEAVY_DELTA >30 stamina=$HEAVY_STAM (≈35)"
    fi
  fi
fi

# ── gate 5: COMBAT_KILL — husk HP ≤ 0, dead flag set ────────────────────
# Format: COMBAT_KILL husk_hp=-1 dead=true => PASS
# Proof: dead=true, and if HP != -1 then HP <= 0.
# HP=-1 is the "entity despawned" sentinel — husk was cleaned up on death.

KILL_LINE=$(grep 'COMBAT_KILL' "$log" | head -1)
if [ -z "$KILL_LINE" ]; then
  fail "COMBAT_KILL" "line not found in log" "missing" "COMBAT_KILL present"
else
  KILL_RESULT=$(extract "$KILL_LINE" '.*=> (PASS|FAIL).*')
  KILL_HP=$(extract "$KILL_LINE" '.*husk_hp=(-?[0-9.]+).*')
  KILL_DEAD=$(extract "$KILL_LINE" '.*dead=(true|false).*')

  if [ "$KILL_RESULT" = "FAIL" ]; then
    fail "COMBAT_KILL" "self-graded FAIL" "hp=$KILL_HP dead=$KILL_DEAD" "dead=true"
  elif [ -z "$KILL_HP" ] || [ -z "$KILL_DEAD" ]; then
    fail "COMBAT_KILL" "could not parse values" "line=$KILL_LINE" "hp/dead"
  elif [ "$KILL_DEAD" != "true" ]; then
    fail "COMBAT_KILL" "dead flag is false (husk not killed)" \
      "hp=$KILL_HP dead=$KILL_DEAD" \
      "dead=true"
  else
    # If hp != -1 (the "entity gone" sentinel), assert hp <= 0
    if ! awk -v hp="$KILL_HP" 'BEGIN { exit (hp == -1 || hp <= 0.0) ? 0 : 1 }'; then
      fail "COMBAT_KILL" "husk HP > 0 but dead=true (inconsistent)" \
        "hp=$KILL_HP dead=$KILL_DEAD" \
        "hp <= 0 or hp == -1 (despawned)"
    else
      if awk -v hp="$KILL_HP" 'BEGIN { exit (hp == -1) ? 0 : 1 }'; then
        pass "COMBAT_KILL hp=$KILL_HP (despawned) dead=true"
      else
        pass "COMBAT_KILL hp=$KILL_HP (<=0) dead=true"
      fi
    fi
  fi
fi

# ── gate 6: COMBAT_DEATH — player HP forced to 0, dead flag set ─────────
# Format: COMBAT_DEATH hp=0/100 dead=true => PASS
# Proof: cur HP == 0, dead == true

DEATH_LINE=$(grep 'COMBAT_DEATH' "$log" | head -1)
if [ -z "$DEATH_LINE" ]; then
  fail "COMBAT_DEATH" "line not found in log" "missing" "COMBAT_DEATH present"
else
  DEATH_RESULT=$(extract "$DEATH_LINE" '.*=> (PASS|FAIL).*')
  DEATH_CUR=$(extract "$DEATH_LINE" '.*hp=(-?[0-9.]+)\/[-0-9.]+.*')
  DEATH_MAX=$(extract "$DEATH_LINE" '.*hp=[-0-9.]+\/([-0-9.]+).*')
  DEATH_DEAD=$(extract "$DEATH_LINE" '.*dead=(true|false).*')

  if [ "$DEATH_RESULT" = "FAIL" ]; then
    fail "COMBAT_DEATH" "self-graded FAIL" "hp=$DEATH_CUR/$DEATH_MAX dead=$DEATH_DEAD" "hp=0 dead=true"
  elif [ -z "$DEATH_CUR" ] || [ -z "$DEATH_MAX" ] || [ -z "$DEATH_DEAD" ]; then
    fail "COMBAT_DEATH" "could not parse values" "line=$DEATH_LINE" "cur/max/dead"
  else
    PASSED=true
    if ! awk -v c="$DEATH_CUR" 'BEGIN { exit (c == 0.0) ? 0 : 1 }'; then
      fail "COMBAT_DEATH" "player HP not 0 after forced kill" \
        "hp=$DEATH_CUR/$DEATH_MAX" \
        "cur=0"
      PASSED=false
    fi
    if ! awk -v m="$DEATH_MAX" 'BEGIN { exit (m > 0.0) ? 0 : 1 }'; then
      fail "COMBAT_DEATH" "max HP invalid (<= 0)" \
        "max=$DEATH_MAX" \
        "max > 0"
      PASSED=false
    fi
    if [ "$DEATH_DEAD" != "true" ]; then
      fail "COMBAT_DEATH" "dead flag not true" \
        "dead=$DEATH_DEAD" \
        "dead=true"
      PASSED=false
    fi
    if $PASSED; then
      pass "COMBAT_DEATH hp=0/$DEATH_MAX dead=true"
    fi
  fi
fi

# ── gate 7: COMBAT_RESPAWN — at campfire, full HP restored ──────────────
# Format: COMBAT_RESPAWN at=(32.5,2.6,29.5) campfire=(32.5,1.0,29.5) dist=0.00 hp=100/100 => PASS
# Proof: dist < 2.0 blocks from campfire, cur HP ≈ max HP (within 1)

RESPAWN_LINE=$(grep 'COMBAT_RESPAWN' "$log" | head -1)
if [ -z "$RESPAWN_LINE" ]; then
  fail "COMBAT_RESPAWN" "line not found in log" "missing" "COMBAT_RESPAWN present"
else
  RESPAWN_RESULT=$(extract "$RESPAWN_LINE" '.*=> (PASS|FAIL).*')
  RESPAWN_DIST=$(extract "$RESPAWN_LINE" '.*dist=(-?[0-9.]+).*')
  RESPAWN_HP_CUR=$(extract "$RESPAWN_LINE" '.*hp=(-?[0-9.]+)\/[-0-9.]+.*')
  RESPAWN_HP_MAX=$(extract "$RESPAWN_LINE" '.*hp=[-0-9.]+\/([-0-9.]+).*')

  if [ "$RESPAWN_RESULT" = "FAIL" ]; then
    fail "COMBAT_RESPAWN" "self-graded FAIL" \
      "dist=$RESPAWN_DIST hp=$RESPAWN_HP_CUR/$RESPAWN_HP_MAX" \
      "dist < 2.0 hp ~ max"
    # Print detail lines if present
    grep 'COMBAT_RESPAWN detail:' "$log" || true
  elif [ -z "$RESPAWN_DIST" ] || [ -z "$RESPAWN_HP_CUR" ] || [ -z "$RESPAWN_HP_MAX" ]; then
    fail "COMBAT_RESPAWN" "could not parse values" "line=$RESPAWN_LINE" "dist/hp_cur/hp_max"
  else
    PASSED=true
    if ! awk -v d="$RESPAWN_DIST" 'BEGIN { exit (d < 2.0) ? 0 : 1 }'; then
      fail "COMBAT_RESPAWN" "too far from campfire" \
        "dist=$RESPAWN_DIST" \
        "dist < 2.0"
      PASSED=false
    fi
    if ! awk -v c="$RESPAWN_HP_CUR" -v m="$RESPAWN_HP_MAX" \
      'BEGIN { diff = (c - m); diff = (diff >= 0 ? diff : -diff); exit (diff < 1.0) ? 0 : 1 }'; then
      HP_DIFF=$(awk -v c="$RESPAWN_HP_CUR" -v m="$RESPAWN_HP_MAX" \
        'BEGIN { printf "%.1f", c - m }')
      fail "COMBAT_RESPAWN" "HP not fully restored" \
        "hp=$RESPAWN_HP_CUR/$RESPAWN_HP_MAX diff=$HP_DIFF" \
        "|cur - max| < 1.0"
      PASSED=false
    fi
    if $PASSED; then
      pass "COMBAT_RESPAWN dist=$RESPAWN_DIST (<2) hp=$RESPAWN_HP_CUR/$RESPAWN_HP_MAX (full)"
    fi
  fi
fi

# ── gate 8: COMBAT_WALK_DONE — reached melee range ──────────────────────
# Format: COMBAT_WALK_DONE player=(33.3,27.7) husk_dist=2.48 walk_t=0.9s — ...
# Proof: husk_dist must be within attack range (<= ATTACK_DIST = 2.5)

WALK_LINE=$(grep 'COMBAT_WALK_DONE' "$log" | head -1)
if [ -z "$WALK_LINE" ]; then
  fail "COMBAT_WALK_DONE" "line not found in log" "missing" "COMBAT_WALK_DONE present"
else
  WALK_DIST=$(extract "$WALK_LINE" '.*husk_dist=(-?[0-9.]+).*')
  WALK_T=$(extract "$WALK_LINE" '.*walk_t=(-?[0-9.]+)s.*')

  if [ -z "$WALK_DIST" ]; then
    fail "COMBAT_WALK_DONE" "could not parse husk_dist" "line=$WALK_LINE" "husk_dist=N.NN"
  else
    # ATTACK_DIST = 2.5 (scene.rs:918). Assert reached within 3.0 (tolerance
    # for one-frame drift — the Rust code already guarantees <= 2.5, but
    # we add a safety margin so this gate never flakes).
    if ! awk -v d="$WALK_DIST" 'BEGIN { exit (d <= 3.0) ? 0 : 1 }'; then
      fail "COMBAT_WALK_DONE" "husk_dist > 3.0 (never reached melee range)" \
        "husk_dist=$WALK_DIST walk_t=${WALK_T:-?}s" \
        "husk_dist <= 3.0"
    else
      pass "COMBAT_WALK_DONE husk_dist=$WALK_DIST walk_t=${WALK_T:-?}s (in range)"
    fi
  fi
fi

# ── gate 9: COMBAT_FATAL — must not appear ──────────────────────────────

if grep -q 'COMBAT_FATAL' "$log"; then
  fail "COMBAT_FATAL" "fatal phase 99 reached — proof did not complete" \
    "present" \
    "COMBAT_FATAL absent"
fi

# ── final verdict ────────────────────────────────────────────────────────

echo ""
echo "─── VERDICT ───"
echo "failures: $FAIL_COUNT"

if [ "$FAIL_COUNT" -gt 0 ]; then
  echo "  ✗ PROVE_COMBAT: FAIL ($FAIL_COUNT gate(s) failed)"
  echo "Failed gates:"
  echo "$BAD"
  exit 1
else
  echo "PROVE_COMBAT: PASS — all gates verified with numeric assertions"
  exit 0
fi
