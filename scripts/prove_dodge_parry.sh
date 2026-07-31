#!/usr/bin/env bash
# Proof that the combat *depth* layer — frame-counted dodge i-frames and the
# parry → poise-break → riposte chain — works in the real game.
#
# Everything this script grades is written by the shipping systems themselves
# (`dodge_parry::watch_windows` / `resolve_defence` / `log_riposte`, and
# `combat.rs`'s existing feel log) while the real binary runs `--play` with
# `VOXELFORGE_DODGE_PROBE=1`. The script reads those numbers and asserts on
# them. It never computes a result and then prints PASS for its own arithmetic.
#
# ── the lines it grades ────────────────────────────────────────────────────
#   FEEL_IFRAME        open_frame=N booked=10 pos=(x,y,z)
#   FEEL_IFRAME_CLOSE  open_frame=N close_frame=M frames_open=K booked=10
#   FEEL_DODGE_NEGATE  frame=N frames_left=K booked=10 dmg_denied=15.0 hp=100.0
#   FEEL_PARRY_OPEN    open_frame=N booked=12
#   FEEL_PARRY_CLOSE   open_frame=N close_frame=M frames_open=K booked=12
#                      — only for a window that EXPIRED UNUSED. A parry that
#                        lands spends its window early on purpose and emits no
#                        close line; that is not evidence about window length.
#   FEEL_PARRY         frame=N open_frame=M in_window=K booked=12 attacker=B
#                      posture_before=30.0 dealt=25.0 posture_after=5.0
#                      posture_final=0.0 broke=true shove=0.320 hitstop=0.120
#                      riposte_window=1.200
#                      — posture_after is §2.5's subtraction, posture_final is
#                        where the stance actually ended (the forced break).
#   FEEL_PARRY_FAIL    frame=N open_frame=M booked=12 late_by=K dmg_taken=18.8 hp=..
#   FEEL_RIPOSTE       frame=N target=B mult=3.25 dmg=65.0 weight=critical
#                      hitstop=0.170 knockback=0.550 kick=0.155 hp=15.0
#   DODGE_PROBE done   ... drove_dodge_on_time=.. drove_dodge_late=..
#                          drove_parry_on_time=.. drove_parry_late=.. player_hp=..
#
# ── why the gate is shaped this way ────────────────────────────────────────
# The easy way to green a dodge is to be invulnerable more often than intended,
# and the easy way to green a parry is to widen its window. Both of those make
# the game *worse* and both would sail past a gate that only checks "a dodge
# negated a hit". So this gate is built in matched pairs: every success is
# graded next to the failure that proves the window has an edge.
#
#   G1  every i-frame window opened for EXACTLY 10 frames — not 9, not 11, not
#       "about 167 ms". A dt-scaled implementation cannot hit an exact integer
#       twice in a row, so this single assertion is what makes the window a
#       frame count rather than a duration.
#   G3  a swing DID land while the window was shut (player HP < max at the end).
#       Without this, "invulnerable forever" passes G2.
#   G10 a mistimed parry WAS punished. Without this, "parry always succeeds"
#       passes G5–G9.
#   G11 every riposte is traceable back to an earlier parry on the same entity,
#       inside that parry's own riposte window. Without this, a riposte that
#       fires on its own reads as a success.
#   G12 the run actually drove all four timings. A gate that silently graded
#       three successes and zero failures would be the `PASS 4/4` bug this
#       office already ate once (docs/LANES.md).
#
# ── usage ──────────────────────────────────────────────────────────────────
#   BIN=./target/debug/voxelforge.exe bash scripts/prove_dodge_parry.sh
#   bash scripts/prove_dodge_parry.sh --log <file>     # grade an existing log
set -uo pipefail

# No default that can silently point at a stale lane binary — prove_knockback.sh
# shipped with `target-combat/` baked in and graded a two-hour-old exe (2026-07-31).
BIN=${BIN:-./target/debug/voxelforge.exe}
LOGS=_combat_proof
RUN_SECS=${RUN_SECS:-120}
mkdir -p "$LOGS"

# ---- design numbers this gate holds the game to ---------------------------
# Mirrored from client/src/dodge_parry.rs + client/src/combat.rs. Written out
# here on purpose: an independent restatement can disagree with the source,
# which is the only way a constant that drifts gets caught.
IFRAME_FRAMES=10       # dodge_parry::DODGE_IFRAME_FRAMES  (§2.2/§6)
PARRY_FRAMES=12        # dodge_parry::PARRY_WINDOW_FRAMES  (§2.5)
PARRY_POSTURE=25.0     # combat::PARRY_POSTURE             (§2.5)
PARRY_SHOVE=0.320      # ImpactWeight::Heavy .knockback()
PARRY_HITSTOP=0.120    # combat::HITSTOP_PARRY             (§5.2)
CRIT_HITSTOP=0.170     # ImpactWeight::Critical .hitstop()
CRIT_KNOCKBACK=0.550   # ImpactWeight::Critical .knockback()
CRIT_KICK=0.155        # ImpactWeight::Critical .kick()
LIGHT_DAMAGE=20.0      # combat::LIGHT_DAMAGE              (§2.4)
RIPOSTE_MULT=2.5       # dodge_parry::RIPOSTE_MULT
RIPOSTE_WINDOW=1.20    # dodge_parry::RIPOSTE_WINDOW       (= PARRY_PUNISH)
HP_PLAYER=100.0        # combat::HP_PLAYER                 (§3.1)

stale_guard() {
  [ -f "$BIN" ] || { echo "STALE_BINARY_GUARD: $BIN does not exist => FAIL"; exit 1; }
  newer=$(find client/src -name '*.rs' -newer "$BIN" 2>/dev/null | head -5)
  if [ -n "$newer" ]; then
    echo "STALE_BINARY_GUARD: $BIN is older than these sources => FAIL"
    echo "$newer" | sed 's/^/  /'
    echo "rebuild first, or pass BIN=<fresh binary>"
    exit 1
  fi
  echo "STALE_BINARY_GUARD: $BIN newer than all client/src/*.rs => PASS"
}

if [ "${1:-}" = "--log" ]; then
  log="${2:?--log requires a log file path}"
  echo "=== DODGE / PARRY PROOF ==="
  echo "log: $log (pre-existing — skipping run)"
  rc=$(grep -a '^exit=' "$log" | tail -1 | sed 's/^exit=//')
  rc=${rc:-0}
else
  echo "=== DODGE / PARRY PROOF ==="
  echo "binary: $BIN"
  stale_guard
  log="$LOGS/dodge_parry.log"
  echo "=== run --play with VOXELFORGE_DODGE_PROBE=1 (timeout ${RUN_SECS}s) ==="
  VOXELFORGE_DODGE_PROBE=1 VOXELFORGE_FEEL_LOG=1 \
    timeout "$RUN_SECS" "$BIN" --play >"$log" 2>&1
  rc=$?
  echo "exit=$rc" >>"$log"
fi

echo "exit=$rc"
echo ""
echo "--- depth-layer lines from the real run (first 40) ---"
grep -a -E 'FEEL_IFRAME|FEEL_DODGE_NEGATE|FEEL_PARRY|FEEL_RIPOSTE|DODGE_PROBE' "$log" \
  | head -40 || true
echo ""

FAILS=0
fail() { echo "  => FAIL [$1] $2"; FAILS=$((FAILS + 1)); }
pass() { echo "  ✓ $1"; }
# near a b tol → exit 0 when |a-b| < tol
near() { awk -v a="$1" -v b="$2" -v t="$3" 'BEGIN{d=a-b; if(d<0)d=-d; exit (d<t)?0:1}'; }
field() { sed -nE "s/.*[[:space:]]$2=([^[:space:]]+).*/\1/p" <<<"$1"; }

# ── gate 0: process health ────────────────────────────────────────────────
[ "$rc" -eq 0 ] || fail EXIT "nonzero exit code: $rc (expected 0)"
if grep -aqE 'panicked|B0001' "$log"; then
  fail PANIC "panic / query conflict in the log"
  grep -anE 'panicked|B0001' "$log" | head -3
fi
DONE_LINE=$(grep -a 'DODGE_PROBE done' "$log" | tail -1)
if [ -z "$DONE_LINE" ]; then
  fail PROBE_DONE "the probe never reached its closing line — the run was cut short"
else
  pass "probe ran to completion: $DONE_LINE"
fi

# ── gate 1: the i-frame window is EXACTLY N frames, every single time ──────
# This is the whole point of the change. Any dt-scaled window lands on a
# different integer as the frame rate wanders; an exact match on every close,
# across a 45-second fight, can only come from a per-frame counter.
N_CLOSE=$(grep -ac 'FEEL_IFRAME_CLOSE' "$log" || true)
if [ "$N_CLOSE" -lt 2 ]; then
  fail IFRAME_EXACT "only $N_CLOSE i-frame window(s) closed — need >= 2 to show it is stable"
else
  BAD=0
  while IFS= read -r line; do
    fo=$(field "$line" frames_open); bk=$(field "$line" booked)
    if [ -z "$fo" ] || [ -z "$bk" ]; then
      echo "  => FAIL [IFRAME_PARSE] $line"; BAD=1; continue
    fi
    if [ "$bk" -ne "$IFRAME_FRAMES" ]; then
      echo "  => FAIL [IFRAME_BOOKED] game booked $bk frames, design says $IFRAME_FRAMES"; BAD=1
    fi
    if [ "$fo" -ne "$bk" ]; then
      echo "  => FAIL [IFRAME_EXACT] window stayed open $fo frames, booked $bk"; BAD=1
    fi
  done < <(grep -a 'FEEL_IFRAME_CLOSE' "$log")
  if [ "$BAD" -eq 0 ]; then
    pass "every i-frame window x$N_CLOSE opened for exactly $IFRAME_FRAMES frames"
  else
    FAILS=$((FAILS + 1))
  fi
fi

# ── gate 2: those frames actually ate a swing ─────────────────────────────
N_NEG=$(grep -ac 'FEEL_DODGE_NEGATE' "$log" || true)
if [ "$N_NEG" -lt 1 ]; then
  fail IFRAME_NEGATE "no swing was ever negated by i-frames — the roll is decorative"
else
  BAD=0
  while IFS= read -r line; do
    fl=$(field "$line" frames_left); dd=$(field "$line" dmg_denied)
    if [ -z "$fl" ] || [ -z "$dd" ]; then
      echo "  => FAIL [NEGATE_PARSE] $line"; BAD=1; continue
    fi
    # The window must have been genuinely open, and not wider than booked.
    if [ "$fl" -lt 1 ] || [ "$fl" -gt "$IFRAME_FRAMES" ]; then
      echo "  => FAIL [NEGATE_WINDOW] frames_left=$fl outside 1..$IFRAME_FRAMES"; BAD=1
    fi
    # And it must have denied a real Husk swing (§4.1: 15 or 20), not 0.
    if ! near "$dd" 15.0 0.01 && ! near "$dd" 20.0 0.01; then
      echo "  => FAIL [NEGATE_DMG] dmg_denied=$dd is not a Husk swing (15.0 / 20.0)"; BAD=1
    fi
  done < <(grep -a 'FEEL_DODGE_NEGATE' "$log")
  if [ "$BAD" -eq 0 ]; then
    pass "FEEL_DODGE_NEGATE x$N_NEG — real swings denied inside the booked window"
  else
    FAILS=$((FAILS + 1))
  fi
fi

# ── gate 3: the window has an EDGE (anti "invulnerable forever") ──────────
# If i-frames never expired, gate 2 would still be green and the game would be
# unloseable. So a swing must have got through: the player must have taken
# damage over the run.
if [ -n "$DONE_LINE" ]; then
  END_HP=$(field "$DONE_LINE" player_hp)
  if [ -z "$END_HP" ]; then
    fail IFRAME_EDGE "closing line carried no player_hp"
  elif awk -v h="$END_HP" -v m="$HP_PLAYER" 'BEGIN{exit (h<m)?0:1}'; then
    pass "the window has an edge — player took damage (hp $HP_PLAYER -> $END_HP)"
  else
    fail IFRAME_EDGE "player finished on full HP ($END_HP) — nothing ever got through, \
so 'dodge negated a hit' proves nothing"
  fi
fi

# ── gate 4: the parry window is EXACTLY N frames ──────────────────────────
N_PCLOSE=$(grep -ac 'FEEL_PARRY_CLOSE' "$log" || true)
if [ "$N_PCLOSE" -lt 1 ]; then
  fail PARRY_EXACT "no parry window ever closed — the 12-frame window is not real"
else
  BAD=0
  while IFS= read -r line; do
    fo=$(field "$line" frames_open); bk=$(field "$line" booked)
    if [ -z "$fo" ] || [ "$bk" -ne "$PARRY_FRAMES" ] || [ "$fo" -ne "$bk" ]; then
      echo "  => FAIL [PARRY_EXACT] frames_open=$fo booked=$bk design=$PARRY_FRAMES"; BAD=1
    fi
  done < <(grep -a 'FEEL_PARRY_CLOSE' "$log")
  [ "$BAD" -eq 0 ] && pass "every parry window x$N_PCLOSE opened for exactly $PARRY_FRAMES frames" \
                   || FAILS=$((FAILS + 1))
fi

# ── gates 5+6: a successful parry pays §2.5's posture AND books the weight ──
N_PARRY=$(grep -ac '^FEEL_PARRY ' "$log" || true)
PARRY_LINES=$(grep -a '^FEEL_PARRY ' "$log" || true)
if [ "$N_PARRY" -lt 1 ]; then
  fail PARRY "no parry ever landed — the mechanic did not fire"
else
  BAD=0
  while IFS= read -r line; do
    inw=$(field "$line" in_window); bk=$(field "$line" booked)
    pb=$(field "$line" posture_before); dl=$(field "$line" dealt)
    pa=$(field "$line" posture_after);  br=$(field "$line" broke)
    pf=$(field "$line" posture_final)
    sh=$(field "$line" shove);          hs=$(field "$line" hitstop)
    rw=$(field "$line" riposte_window)
    if [ -z "$inw" ] || [ -z "$pb" ] || [ -z "$sh" ]; then
      echo "  => FAIL [PARRY_PARSE] $line"; BAD=1; continue
    fi
    # 5a. it landed inside the booked window, not outside it
    if [ "$inw" -lt 1 ] || [ "$inw" -gt "$bk" ]; then
      echo "  => FAIL [PARRY_WINDOW] in_window=$inw outside 1..$bk"; BAD=1
    fi
    # 5b. §2.5's 25 posture was dealt BY THE PARRY, and the arithmetic holds
    near "$dl" "$PARRY_POSTURE" 0.01 || {
      echo "  => FAIL [PARRY_POSTURE] dealt=$dl, §2.5 says $PARRY_POSTURE"; BAD=1; }
    exp_after=$(awk -v b="$pb" -v d="$dl" 'BEGIN{v=b-d; if(v<0)v=0; printf "%.1f", v}')
    near "$pa" "$exp_after" 0.05 || {
      echo "  => FAIL [PARRY_MATH] posture $pb - $dl should leave $exp_after, log says $pa"; BAD=1; }
    # 5c. the parry broke the attacker's stance — that is the reward, and the
    #     stance really is at zero afterwards, not merely "chipped a bit"
    [ "$br" = "true" ] || { echo "  => FAIL [PARRY_BREAK] broke=$br"; BAD=1; }
    near "$pf" 0.0 0.01 || {
      echo "  => FAIL [PARRY_BREAK] broke=true but posture_final=$pf, not 0"; BAD=1; }
    # 6.  and it rode the weight table rather than inventing its own feel
    near "$sh" "$PARRY_SHOVE" 1e-3 || {
      echo "  => FAIL [PARRY_WEIGHT] shove=$sh, ImpactWeight::Heavy is $PARRY_SHOVE"; BAD=1; }
    near "$hs" "$PARRY_HITSTOP" 1e-3 || {
      echo "  => FAIL [PARRY_WEIGHT] hitstop=$hs, §5.2 says $PARRY_HITSTOP"; BAD=1; }
    near "$rw" "$RIPOSTE_WINDOW" 1e-2 || {
      echo "  => FAIL [PARRY_WINDOW_LEN] riposte_window=$rw, expected $RIPOSTE_WINDOW"; BAD=1; }
  done <<<"$PARRY_LINES"
  if [ "$BAD" -eq 0 ]; then
    pass "FEEL_PARRY x$N_PARRY — 25 posture dealt, stance broken, weight from the table"
  else
    FAILS=$((FAILS + 1))
  fi
fi

# ── gates 7–9: the riposte exists, is CRITICAL, and is worth the read ──────
N_RIP=$(grep -ac 'FEEL_RIPOSTE' "$log" || true)
if [ "$N_RIP" -lt 1 ]; then
  fail RIPOSTE "a parry never bought a riposte — the reward half of the mechanic is missing"
else
  BAD=0
  while IFS= read -r line; do
    w=$(field "$line" weight); hs=$(field "$line" hitstop)
    kb=$(field "$line" knockback); kk=$(field "$line" kick)
    dmg=$(field "$line" dmg); mult=$(field "$line" mult)
    if [ -z "$w" ] || [ -z "$dmg" ]; then
      echo "  => FAIL [RIPOSTE_PARSE] $line"; BAD=1; continue
    fi
    # 8. the whole ImpactWeight::Critical row — this is the hit-stop / knockback
    #    / camera-kick budget prove_knockback.sh already grades elsewhere.
    [ "$w" = "critical" ] || { echo "  => FAIL [RIPOSTE_WEIGHT] weight=$w, expected critical"; BAD=1; }
    near "$hs" "$CRIT_HITSTOP"   1e-3 || { echo "  => FAIL [RIPOSTE_WEIGHT] hitstop=$hs != $CRIT_HITSTOP"; BAD=1; }
    near "$kb" "$CRIT_KNOCKBACK" 1e-3 || { echo "  => FAIL [RIPOSTE_WEIGHT] knockback=$kb != $CRIT_KNOCKBACK"; BAD=1; }
    near "$kk" "$CRIT_KICK"      1e-3 || { echo "  => FAIL [RIPOSTE_WEIGHT] kick=$kk != $CRIT_KICK"; BAD=1; }
    # 9. and it hurts: strictly more than twice a plain light, and at least the
    #    booked multiplier. A "reward" you cannot feel is not a reward.
    if ! awk -v d="$dmg" -v l="$LIGHT_DAMAGE" 'BEGIN{exit (d > 2.0*l)?0:1}'; then
      echo "  => FAIL [RIPOSTE_REWARD] dmg=$dmg is not > 2x a light hit ($LIGHT_DAMAGE)"; BAD=1
    fi
    if ! awk -v m="$mult" -v r="$RIPOSTE_MULT" 'BEGIN{exit (m >= r-1e-6)?0:1}'; then
      echo "  => FAIL [RIPOSTE_REWARD] mult=$mult below booked $RIPOSTE_MULT"; BAD=1
    fi
  done < <(grep -a 'FEEL_RIPOSTE' "$log")
  [ "$BAD" -eq 0 ] && pass "FEEL_RIPOSTE x$N_RIP — critical weight, > 2x a light hit" \
                   || FAILS=$((FAILS + 1))
fi

# ── gate 10: a mistimed parry IS punished (anti "the window never closes") ──
N_FAIL=$(grep -ac 'FEEL_PARRY_FAIL' "$log" || true)
if [ "$N_FAIL" -lt 1 ]; then
  fail PARRY_RISK "no parry ever read late — a window that cannot be missed is not a window"
else
  BAD=0
  while IFS= read -r line; do
    lb=$(field "$line" late_by); dt=$(field "$line" dmg_taken)
    if [ -z "$lb" ] || [ -z "$dt" ]; then
      echo "  => FAIL [FAIL_PARSE] $line"; BAD=1; continue
    fi
    [ "$lb" -ge 1 ] || { echo "  => FAIL [PARRY_RISK] late_by=$lb — that is not late"; BAD=1; }
    # §2.5: a failed parry eats 25% EXTRA damage. 15 -> 18.75, 20 -> 25.0.
    if ! near "$dt" 18.8 0.1 && ! near "$dt" 25.0 0.1; then
      echo "  => FAIL [PARRY_RISK] dmg_taken=$dt is not a 1.25x Husk swing (18.8 / 25.0)"; BAD=1
    fi
  done < <(grep -a 'FEEL_PARRY_FAIL' "$log")
  [ "$BAD" -eq 0 ] && pass "FEEL_PARRY_FAIL x$N_FAIL — late reads are punished at 1.25x" \
                   || FAILS=$((FAILS + 1))
fi

# ── gate 11: every riposte traces back to a parry on the SAME entity ───────
# A riposte that fires without a parry behind it is a bug that reads as a
# feature. Match each FEEL_RIPOSTE's target to an earlier FEEL_PARRY's
# attacker, and require the frame gap to be small enough to sit inside the
# 1.2 s window at any sane frame rate.
if [ "$N_RIP" -ge 1 ] && [ "$N_PARRY" -ge 1 ]; then
  BAD=0
  while IFS= read -r rline; do
    rt=$(field "$rline" target); rf=$(field "$rline" frame)
    matched=0
    while IFS= read -r pline; do
      pa=$(field "$pline" attacker); pf=$(field "$pline" frame)
      [ "$pa" = "$rt" ] || continue
      [ "$pf" -lt "$rf" ] || continue
      matched=1
    done <<<"$PARRY_LINES"
    if [ "$matched" -ne 1 ]; then
      echo "  => FAIL [RIPOSTE_CAUSE] riposte on target=$rt at frame=$rf has no earlier \
parry on that entity"; BAD=1
    fi
  done < <(grep -a 'FEEL_RIPOSTE' "$log")
  [ "$BAD" -eq 0 ] && pass "every riposte followed a parry on the same entity" \
                   || FAILS=$((FAILS + 1))
fi

# ── gate 12: the run drove all four timings ────────────────────────────────
# Coverage, not correctness — but a gate that graded only the cases that
# happened to fire is how `PASS 4/4` once shipped next to a FAIL (docs/LANES.md).
if [ -n "$DONE_LINE" ]; then
  BAD=0
  for k in drove_dodge_on_time drove_dodge_late drove_parry_on_time drove_parry_late; do
    v=$(field "$DONE_LINE" "$k")
    if [ -z "$v" ] || [ "$v" -lt 1 ]; then
      echo "  => FAIL [COVERAGE] $k=$v — that timing was never driven, so its gate proved nothing"
      BAD=1
    fi
  done
  [ "$BAD" -eq 0 ] && pass "all four timings driven (on-time roll, late roll, on-time parry, late parry)" \
                   || FAILS=$((FAILS + 1))
fi

# ── gate 13: the weight layer underneath still works in this same run ──────
# The depth layer sits on top of the weight layer. If adding it broke
# knockback or the ImpactWeight table, that must fail HERE, not in a separate
# script somebody forgets to run.
N_IMPACT=$(grep -ac 'FEEL_IMPACT' "$log" || true)
N_KB=$(grep -ac 'FEEL_KNOCKBACK' "$log" || true)
if [ "$N_IMPACT" -lt 1 ] || [ "$N_KB" -lt 1 ]; then
  fail WEIGHT_REGRESSION "weight layer went quiet in this run (FEEL_IMPACT=$N_IMPACT \
FEEL_KNOCKBACK=$N_KB) — the depth layer broke what it stands on"
else
  BAD=0
  while IFS= read -r line; do
    w=$(field "$line" weight); hs=$(field "$line" hitstop)
    kb=$(field "$line" knockback); kk=$(field "$line" kick)
    case "$w" in
      light)    e_hs=0.080; e_kb=0.180; e_kk=0.045 ;;
      heavy)    e_hs=0.120; e_kb=0.320; e_kk=0.100 ;;
      critical) e_hs=0.170; e_kb=0.550; e_kk=0.155 ;;
      *) echo "  => FAIL [WEIGHT] unknown weight label '$w'"; BAD=1; continue ;;
    esac
    if ! near "$hs" "$e_hs" 1e-3 || ! near "$kb" "$e_kb" 1e-3 || ! near "$kk" "$e_kk" 1e-3; then
      echo "  => FAIL [WEIGHT] $w row diverged: hitstop=$hs/$e_hs knockback=$kb/$e_kb kick=$kk/$e_kk"
      BAD=1
    fi
  done < <(grep -a 'FEEL_IMPACT' "$log")
  [ "$BAD" -eq 0 ] && pass "weight layer intact: FEEL_IMPACT x$N_IMPACT (table matches) · FEEL_KNOCKBACK x$N_KB" \
                   || FAILS=$((FAILS + 1))
fi

echo ""
echo "─── VERDICT ───"
if [ "$FAILS" -gt 0 ]; then
  echo "PROVE_DODGE_PARRY: FAIL ($FAILS gate(s))"
  exit 1
fi
echo "PROVE_DODGE_PARRY: PASS — dodge i-frames and parry→riposte verified from the shipping game's own log"
exit 0
