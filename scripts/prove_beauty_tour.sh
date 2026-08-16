#!/usr/bin/env bash
# Proof that the `--beauty-tour` cinematic (kevin's app-wiring lane) actually
# runs end to end: the real `beauty_tour` system in `client/src/main.rs` drives
# the real OrbitCam through three authored lighting moods (noon → cool → night),
# recolours the look stack's own sun/fills/ambient/sky/IBL/exposure per phase,
# sends the phase to `audio.rs` via the `BeautyTourPhase` message (which flips
# the ambient-zone override so the ambience follows the TOUR, not the parked
# player body), captures a still PNG at each of the three stops, and auto-exits.
#
# Same rule as prove_audio.sh / prove_dodge_parry.sh: every PASS below is graded
# off a line `main.rs`/`audio.rs` itself printed, or a file the run itself wrote.
# This script never computes a result and prints PASS for its own arithmetic.
#
# ── the lines it grades (all from client/src/main.rs or client/src/audio.rs) ─
#   BEAUTY_TOUR phase=Noon|Cool|Night t=<secs>   — beauty_tour, on phase change
#   BEAUTY_TOUR shot N -> <dir>/beauty-tour-stop-N.png — still spawn, one per stop
#   BEAUTY_TOUR done t=<secs> => exit             — the tour finished + auto-exit
#   AUDIO_ZONE:Wilds|Village|Campfire t=...       — track_ambient_zone via override
#
# ── why the gate is shaped this way ────────────────────────────────────────
#   G0 panic guard — this exact feature shipped a B0001 query-conflict panic on
#      EVERY boot (not just --beauty-tour), because Bevy validates query access
#      at schedule-build time before any run_if. A run that reaches
#      `BEAUTY_TOUR done` is the positive proof that fix is in the binary.
#   G1 stale-binary guard — a fresh exe is the only way to be sure the panic fix
#      and the three moods are actually in what we just graded.
#   G4 the three stills must be pairwise-distinct bytes — three identical PNGs
#      would mean the moods never actually re-lit the frame (the look stack is
#      apply-once, so a regressed recolour would still "run" and still print
#      phase lines while rendering the same image three times).
#   G5 the ambience override — the tour camera flies AWAY from the campsite; a
#      Campfire zone line at t>=15 is only reachable through the override, never
#      through player position (the body never moves). That is the audio.rs
#      half of the feature, proven rather than assumed.
#
# ── usage ──────────────────────────────────────────────────────────────────
#   BIN=./target-kevin/release/voxelforge.exe bash scripts/prove_beauty_tour.sh
#   SHOTS_DIR=_beauty_tour_shots bash scripts/prove_beauty_tour.sh
#   bash scripts/prove_beauty_tour.sh --log <file>   # grade an existing run log
set -uo pipefail

BIN=${BIN:-./target-kevin/release/voxelforge.exe}
LOGS=_beauty_tour_gate
SHOTS_DIR=${SHOTS_DIR:-_beauty_tour_shots}
RUN_SECS=${RUN_SECS:-120}
mkdir -p "$LOGS" "$SHOTS_DIR"

stale_guard() {
  [ -f "$BIN" ] || { echo "STALE_BINARY_GUARD: $BIN does not exist => FAIL"; exit 1; }
  newer=$(find client/src -name '*.rs' -newer "$BIN" 2>/dev/null | head -5)
  if [ -n "$newer" ]; then
    echo "STALE_BINARY_GUARD: $BIN is older than these sources => FAIL"
    echo "$newer" | sed 's/^/  /'
    echo "rebuild first (CARGO_TARGET_DIR=target-kevin cargo build --bin voxelforge --release), or pass BIN=<fresh binary>"
    exit 1
  fi
  echo "STALE_BINARY_GUARD: $BIN newer than all client/src/*.rs => PASS"
}

if [ "${1:-}" = "--log" ]; then
  log="${2:?--log requires a log file path}"
  echo "=== BEAUTY TOUR PROOF ==="
  echo "log: $log (pre-existing — skipping run)"
  rc=$(grep -a '^exit=' "$log" | tail -1 | sed 's/^exit=//')
  rc=${rc:-0}
else
  echo "=== BEAUTY TOUR PROOF ==="
  echo "binary: $BIN"
  echo "shots dir: $SHOTS_DIR"
  stale_guard
  log="$LOGS/beauty_tour.log"
  echo "=== run voxelforge --beauty-tour --beauty-tour-shots $SHOTS_DIR (timeout ${RUN_SECS}s) ==="
  timeout "$RUN_SECS" "$BIN" --beauty-tour --beauty-tour-shots "$SHOTS_DIR" >"$log" 2>&1
  rc=$?
  echo "exit=$rc" >>"$log"
fi

echo "exit=$rc"
echo ""
echo "--- real beauty-tour evidence lines from the run ---"
grep -a -E '^BEAUTY_TOUR ' "$log" | head -20 || true
echo ""

FAILS=0
fail() { echo "  => FAIL [$1] $2"; FAILS=$((FAILS + 1)); }
pass() { echo "  + $1"; }
note() { echo "  ~ NOTE [$1] $2"; }
field() { sed -nE "s/.*[[:space:]]$2=([^[:space:]]+).*/\1/p" <<<"$1"; }

# ── gate 0: process health + the tour reached its own auto-exit ─────────────
if grep -aqE 'panicked|B0001' "$log"; then
  fail PANIC "panic / query conflict in the log"
  grep -anE 'panicked|B0001' "$log" | head -3
else
  pass "no panic / B0001 query conflict (the beauty_tour ParamSet fix held)"
fi
if [ "$rc" -eq 0 ] && grep -aq 'BEAUTY_TOUR done' "$log"; then
  pass "tour ran to its own auto-exit (BEAUTY_TOUR done, exit=$rc)"
else
  fail RUN_DONE "exit=$rc or no BEAUTY_TOUR done — tour cut short"
fi

# ── gate 1: stale-binary guard already ran (echo above) ──────────────────────
# (done in stale_guard; kept here so --log mode still documents the intent)

# ── gate 2: the three phases, in order, each with a real t= timestamp ────────
phase_t() { grep -a "^BEAUTY_TOUR phase=$1 " "$log" | head -1 | sed -nE "s/.*t=([0-9.]+).*/\1/p"; }
NOON_T=$(phase_t Noon); COOL_T=$(phase_t Cool); NIGHT_T=$(phase_t Night)
if [ -z "$NOON_T" ] || [ -z "$COOL_T" ] || [ -z "$NIGHT_T" ]; then
  fail PHASES "missing phase line(s): noon=$NOON_T cool=$COOL_T night=$NIGHT_T"
else
  ok=1
  awk -v a="$NOON_T" -v b="$COOL_T" -v c="$NIGHT_T" 'BEGIN{exit (a<b && b<c)?0:1}' || ok=0
  awk -v c="$NIGHT_T" 'BEGIN{exit (c>=15.0)?0:1}' || ok=0
  if [ "$ok" -eq 1 ]; then
    pass "phases noon(t=$NOON_T) -> cool(t=$COOL_T) -> night(t=$NIGHT_T), night at t>=15"
  else
    fail PHASES "phase order/timing off: noon=$NOON_T cool=$COOL_T night=$NIGHT_T"
  fi
fi

# ── gate 3: all three stills were requested ─────────────────────────────────
for n in 1 2 3; do
  if grep -aq "BEAUTY_TOUR shot $n ->" "$log"; then
    pass "shot $n requested (BEAUTY_TOUR shot $n -> ...)"
  else
    fail SHOT_$n "shot $n never requested"
  fi
done

# ── gate 4: the three PNGs exist, are non-empty, and pairwise-distinct ───────
declare -A MD5=()
ok=1
for n in 1 2 3; do
  p="$SHOTS_DIR/beauty-tour-stop-$n.png"
  if [ ! -f "$p" ]; then
    fail PNG_$n "$p not written"
    ok=0; continue
  fi
  sz=$(wc -c <"$p" | tr -d ' ')
  if [ "$sz" -lt 1024 ]; then
    fail PNG_$n "$p is $sz bytes — degenerate/empty still"
    ok=0; continue
  fi
  MD5[$n]=$(md5sum "$p" | cut -d' ' -f1)
  pass "$p ($sz bytes, md5 ${MD5[$n]:0:8}…)"
done
if [ "$ok" -eq 1 ] && [ "${MD5[1]}" != "${MD5[2]}" ] && [ "${MD5[1]}" != "${MD5[3]}" ] && [ "${MD5[2]}" != "${MD5[3]}" ]; then
  pass "three stills are pairwise-distinct bytes — the moods actually re-lit the frame"
else
  fail PNG_DISTINCT "stills are not pairwise distinct (or missing) — moods may render identical"
fi

# ── gate 5: ambience follows the TOUR via the audio.rs override ─────────────
# The camera flies away from the campsite while the body parks; a Campfire zone
# at t>=15 (the night phase) is only reachable through the BeautyTourPhase
# override, never through player position. Graded off audio.rs's own line.
CAMP_LINE=$(grep -a '^AUDIO_ZONE:Campfire ' "$log" | tail -1)
VILL_LINE=$(grep -a '^AUDIO_ZONE:Village ' "$log" | tail -1)
if [ -n "$VILL_LINE" ]; then
  tv=$(field "$VILL_LINE" t)
  if awk -v t="$tv" 'BEGIN{exit (t>=5.0)?0:1}'; then
    pass "AUDIO_ZONE:Village at t=$tv (cool phase, override-driven)"
  else
    fail AUDIO_VILLAGE "AUDIO_ZONE:Village at t=$tv — expected >= 5.0 (cool phase)"
  fi
else
  fail AUDIO_VILLAGE "no AUDIO_ZONE:Village line — override never reached Village"
fi
if [ -n "$CAMP_LINE" ]; then
  tc=$(field "$CAMP_LINE" t)
  if awk -v t="$tc" 'BEGIN{exit (t>=15.0)?0:1}'; then
    pass "AUDIO_ZONE:Campfire at t=$tc (night phase, override-driven)"
  else
    fail AUDIO_CAMPFIRE "AUDIO_ZONE:Campfire at t=$tc — expected >= 15.0 (night phase)"
  fi
else
  fail AUDIO_CAMPFIRE "no AUDIO_ZONE:Campfire line — override never reached Campfire"
fi

echo ""
echo "─── VERDICT ───"
if [ "$FAILS" -gt 0 ]; then
  echo "PROVE_BEAUTY_TOUR: FAIL ($FAILS gate(s))"
  exit 1
fi
echo "PROVE_BEAUTY_TOUR: PASS — phases/stills/ambience-override/auto-exit all sourced from main.rs+audio.rs's own lines, three pairwise-distinct stills on disk"
exit 0
