#!/usr/bin/env bash
# Proof that the combat *weight* layer (combat.rs, Poppy's lane) actually moves
# bodies in the real game — not in a unit test, not in a driver's own printout.
#
# Everything this script grades is written by the shipping combat systems
# themselves (`combat_feel_log` / `apply_knockback`) while the real binary runs
# `--combat-demo`. The script only reads those numbers and asserts on them.
#
# The lines it grades:
#   FEEL_IMPACT     pos=(x,y,z) dir=(x,z) weight=light hitstop=0.080 \
#                   knockback=0.180 kick=0.045 target=<bits> attacker=<bits>
#   FEEL_KNOCKBACK  entity=<bits> from=(x,z) to=(x,z) moved=0.180 \
#                   impulse=0.180 slid=0.180 topup=0.000 dir=(x,z)
#
#   `moved` is net displacement; `slid` is what apply_knockback actually pushed.
#   They differ exactly when another system writes the same Transform later in
#   the frame — which is the whole reason both are logged.
#
# The gate:
#   exit 0 · no panic · at least one FEEL_IMPACT · at least one FEEL_KNOCKBACK ·
#   the husk's from!=to (it really was pushed) · moved ≈ impulse ·
#   slid ≈ impulse (the slide delivered) · moved ≈ slid (nothing stole it back) ·
#   the shove ran down the blade direction (dot(dir, to-from) > 0) ·
#   every FEEL_IMPACT's hitstop/knockback/kick match the ImpactWeight table.
#
# Usage:
#   bash scripts/prove_knockback.sh                  # run the binary, then grade
#   bash scripts/prove_knockback.sh --log <file>     # grade an existing log
set -uo pipefail

BIN=${BIN:-./target-combat/debug/voxelforge.exe}
LOGS=_combat_proof
mkdir -p "$LOGS"

if [ "${1:-}" = "--log" ]; then
  log="${2:?--log requires a log file path}"
  echo "=== KNOCKBACK PROOF ==="
  echo "log: $log (pre-existing — skipping run)"
  rc=$(grep -a '^exit=' "$log" | tail -1 | sed 's/^exit=//')
  rc=${rc:-0}
else
  echo "=== KNOCKBACK PROOF ==="
  echo "binary: $BIN"
  log="$LOGS/knockback.log"
  echo "=== run --combat-demo with VOXELFORGE_FEEL_LOG=1 (timeout 180s) ==="
  VOXELFORGE_FEEL_LOG=1 timeout 180 "$BIN" --combat-demo >"$log" 2>&1
  rc=$?
  echo "exit=$rc" >>"$log"
fi

echo "exit=$rc"
echo ""
echo "--- feel lines from the real run ---"
grep -a -E 'FEEL_IMPACT|FEEL_KNOCKBACK|FEEL_STAGGER|FEEL_CAMKICK' "$log" | head -40 || true
echo ""

FAILS=0
fail() { echo "  => FAIL [$1] $2"; FAILS=$((FAILS + 1)); }
pass() { echo "  ✓ $1"; }

# ── gate 0: process health ───────────────────────────────────────────────
[ "$rc" -eq 0 ] || fail EXIT "nonzero exit code: $rc (expected 0)"
if grep -aqE 'panicked|B0001' "$log"; then
  fail PANIC "panic in the log"
  grep -anE 'panicked|B0001' "$log" | head -3
fi

# ── gate 1: an impact was broadcast at all ───────────────────────────────
N_IMPACT=$(grep -ac 'FEEL_IMPACT' "$log" || true)
if [ "$N_IMPACT" -lt 1 ]; then
  fail FEEL_IMPACT "no ImpactEvent reached the log (the weight layer never fired)"
else
  pass "FEEL_IMPACT x$N_IMPACT"
fi

# ── gate 2: every impact's numbers match the ImpactWeight table ──────────
# light 0.080/0.180/0.045 · heavy 0.120/0.320/0.100 · critical 0.170/0.550/0.155
BADW=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  w=$(sed -nE 's/.*weight=([a-z]+).*/\1/p' <<<"$line")
  hs=$(sed -nE 's/.*hitstop=([0-9.]+).*/\1/p' <<<"$line")
  kb=$(sed -nE 's/.*knockback=([0-9.]+).*/\1/p' <<<"$line")
  kk=$(sed -nE 's/.*kick=([0-9.]+).*/\1/p' <<<"$line")
  case "$w" in
    light)    e_hs=0.080; e_kb=0.180; e_kk=0.045 ;;
    heavy)    e_hs=0.120; e_kb=0.320; e_kk=0.100 ;;
    critical) e_hs=0.170; e_kb=0.550; e_kk=0.155 ;;
    *) echo "  => FAIL [WEIGHT] unknown weight label '$w'"; BADW=1; continue ;;
  esac
  if ! awk -v a="$hs" -v b="$e_hs" -v c="$kb" -v d="$e_kb" -v e="$kk" -v f="$e_kk" \
      'function ad(x,y){return (x-y)<0?(y-x):(x-y)}
       BEGIN{exit (ad(a,b)<1e-3 && ad(c,d)<1e-3 && ad(e,f)<1e-3)?0:1}'; then
    echo "  => FAIL [WEIGHT] $w row diverged: hitstop=$hs/$e_hs knockback=$kb/$e_kb kick=$kk/$e_kk"
    BADW=1
  fi
done < <(grep -a 'FEEL_IMPACT' "$log")
if [ "$BADW" -eq 0 ] && [ "$N_IMPACT" -gt 0 ]; then
  pass "every FEEL_IMPACT matches the ImpactWeight table"
elif [ "$BADW" -ne 0 ]; then
  FAILS=$((FAILS + 1))
fi

# ── gate 3: the husk was really pushed ───────────────────────────────────
N_KB=$(grep -ac 'FEEL_KNOCKBACK' "$log" || true)
if [ "$N_KB" -lt 1 ]; then
  fail FEEL_KNOCKBACK "no knockback ever completed — nothing was pushed"
else
  pass "FEEL_KNOCKBACK x$N_KB"
  BADK=0
  while IFS= read -r line; do
    fx=$(sed -nE 's/.*from=\((-?[0-9.]+),(-?[0-9.]+)\).*/\1/p' <<<"$line")
    fz=$(sed -nE 's/.*from=\((-?[0-9.]+),(-?[0-9.]+)\).*/\2/p' <<<"$line")
    tx=$(sed -nE 's/.*to=\((-?[0-9.]+),(-?[0-9.]+)\).*/\1/p' <<<"$line")
    tz=$(sed -nE 's/.*to=\((-?[0-9.]+),(-?[0-9.]+)\).*/\2/p' <<<"$line")
    mv=$(sed -nE 's/.*moved=([0-9.]+).*/\1/p' <<<"$line")
    im=$(sed -nE 's/.*impulse=([0-9.]+).*/\1/p' <<<"$line")
    sl=$(sed -nE 's/.*slid=([0-9.]+).*/\1/p' <<<"$line")
    tu=$(sed -nE 's/.*topup=([0-9.]+).*/\1/p' <<<"$line")
    dx=$(sed -nE 's/.*dir=\((-?[0-9.]+),(-?[0-9.]+)\)$/\1/p' <<<"$line")
    dz=$(sed -nE 's/.*dir=\((-?[0-9.]+),(-?[0-9.]+)\)$/\2/p' <<<"$line")
    if [ -z "$fx" ] || [ -z "$tx" ] || [ -z "$mv" ] || [ -z "$im" ] || [ -z "$dx" ]; then
      echo "  => FAIL [KB_PARSE] could not parse: $line"; BADK=1; continue
    fi
    # 3a. from != to — the body actually changed position.
    if ! awk -v a="$fx" -v b="$fz" -v c="$tx" -v d="$tz" \
        'BEGIN{exit ((a!=c)||(b!=d))?0:1}'; then
      echo "  => FAIL [KB_STATIC] husk did not move: from=($fx,$fz) to=($tx,$tz)"; BADK=1
    fi
    # 3b. moved must be a real distance, and land within 20% of the booked impulse
    #     (the slide is integrated per-frame, so it lands on or just under it).
    if ! awk -v m="$mv" -v i="$im" \
        'BEGIN{exit (m>0.01 && m<=i*1.05 && m>=i*0.80)?0:1}'; then
      echo "  => FAIL [KB_DIST] moved=$mv not within [0.80,1.05]x impulse=$im"; BADK=1
    fi
    # 3b-i. Split the two ways 3b can fail, so a regression names its own cause
    #       instead of being argued about. `slid` is what apply_knockback put
    #       into the transform; `moved` is what was still there at end of frame.
    #         slid  < impulse  -> the slide itself came up short
    #         slid == impulse but moved < slid -> a later system stole it back
    #       (that second one is the husk_ai step-in bug: 0.180 slid, 0.142 kept)
    if [ -n "$sl" ]; then
      if ! awk -v s="$sl" -v i="$im" 'BEGIN{exit (s>=i*0.99 && s<=i*1.05)?0:1}'; then
        echo "  => FAIL [KB_DELIVERED] apply_knockback only slid=$sl of impulse=$im"; BADK=1
      fi
      if ! awk -v s="$sl" -v m="$mv" 'BEGIN{exit ((s-m)<=0.01)?0:1}'; then
        echo "  => FAIL [KB_STOLEN] slid=$sl but only moved=$mv — a later system \
overwrote the shove"; BADK=1
      fi
      echo "    · slid=$sl moved=$mv topup=$tu (topup>0 would mean a frame closed short)"
    fi
    # 3c. the shove ran DOWN THE BLADE: dot(dir, to-from) > 0.
    if ! awk -v a="$fx" -v b="$fz" -v c="$tx" -v d="$tz" -v x="$dx" -v z="$dz" \
        'BEGIN{exit (((c-a)*x + (d-b)*z) > 0)?0:1}'; then
      echo "  => FAIL [KB_DIR] shove not along blade dir=($dx,$dz) delta=($(awk -v a="$fx" -v c="$tx" 'BEGIN{printf "%.3f", c-a}'),$(awk -v b="$fz" -v d="$tz" 'BEGIN{printf "%.3f", d-b}'))"
      BADK=1
    fi
  done < <(grep -a 'FEEL_KNOCKBACK' "$log")
  if [ "$BADK" -eq 0 ]; then
    pass "every knockback moved the husk, the booked distance, down the blade"
  else
    FAILS=$((FAILS + 1))
  fi
fi

echo ""
echo "─── VERDICT ───"
if [ "$FAILS" -gt 0 ]; then
  echo "PROVE_KNOCKBACK: FAIL ($FAILS gate(s))"
  exit 1
fi
echo "PROVE_KNOCKBACK: PASS — knockback verified from the shipping game's own log"
exit 0
