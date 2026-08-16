#!/usr/bin/env bash
# Poppy -- Hour::NIGHT.ev100 7.5 -> 8.6, round 2: the build is GREEN, the frame
# is not. Round 1 (08:29-08:35) got `errors=0` + `Finished perf in 5m 45s` + a
# proven relink, and the run's own log printed
#   LOOK_FILL gen=V3 ambient=26 ... ev100=7.50   (before leg, env lever)
#   LOOK_FILL gen=V3 ambient=26 ... ev100=8.60   (after  leg, NO lever = source)
# so the constant is in the binary. Both legs then died two lines later on
#
#   error[B0001]: Query<(Entity, &mut DirectionalLight, &mut Transform)> in
#   system voxelforge::beauty_tour ... conflicts with a previous system parameter
#
# which is main.rs's beauty-tour system and NOT this lane. `.run_if` does not
# save it: Bevy validates a system's query access at schedule-init, before any
# run condition is consulted, so every boot of `voxelforge.exe` panics whether or
# not `--beauty-tour` is passed. No plate can be shot by anyone until it is fixed.
#
# SO THIS CHAIN RE-SHOOTS, AND ONLY REBUILDS WHEN IT HAS TO. A rebuild is 6
# minutes of a box that six lanes share; a boot is seconds. It re-boots the exe
# it already has, and spends a build slot only after one of the watched files has
# actually been edited.
#
# Verdict rules unchanged: `grep -c '^error'` on the full cargo log, never a tail
# and never a pipe's exit code. Queue rule unchanged: never be the third build.
#
# Usage: bash scripts/_poppy_ev100_night_chain2.sh
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
LOG=$ROOT/_poppy_build.log
LANE=$ROOT/_poppy_ev100night_lane.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OUT=$ROOT/_poppy_ev100night
SHOOT=$ROOT/scripts/_poppy_ev100_night_shoot.cmd
WATCH=(client/src/main.rs client/src/audio.rs client/src/scene.rs client/src/look.rs)
MAXTRY=12

# Live cargo BUILD launches. cargo runs as TWO processes per launch (rustup shim
# + real toolchain exe); counting unique CreationDate does not deduplicate them
# because the timestamps differ in the sub-second fraction -- that is what made
# round 1 read "other builds=2" on a box with one build. Match the toolchain path.
nbuilds() {
  powershell -NoProfile -Command \
    "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\" | Where-Object { \$_.CommandLine -like '*\\bin\\cargo.exe*' }).Count" \
    2>/dev/null | tr -d '\r' | tr -dc '0-9'
}
stamp() { date '+%H:%M:%S'; }
sigs()  { for f in "${WATCH[@]}"; do printf '%s:%s ' "$f" "$(date -r "$f" +%s 2>/dev/null || echo 0)"; done; }

# Block until one of the watched files is edited by its owner. Returns 1 if the
# wait timed out, so the caller can decide whether a blind retry is worth a slot.
wait_for_edit() {
  local before=$1 i
  for i in $(seq 1 90); do   # 90 x 20s = 30 min
    sleep 20
    if [ "$(sigs)" != "$before" ]; then
      echo "  $(stamp) a watched file moved:"
      for f in "${WATCH[@]}"; do
        printf '    %-22s %s\n' "$f" "$(date -r "$f" '+%H:%M:%S')"
      done
      return 0
    fi
    [ $((i % 6)) -eq 0 ] && echo "  $(stamp) still waiting on another lane ($((i * 20))s)"
  done
  echo "  $(stamp) 30 min with no edit to any watched file"
  return 1
}

need_build=0
echo "=== ev100 night chain2 started $(stamp) ==="
echo "exe on disk: $(date -r "$EXE" '+%F %T')  sha $(sha256sum "$EXE" | awk '{print $1}')"

for try in $(seq 1 $MAXTRY); do
  echo
  echo "=== attempt $try/$MAXTRY  (rebuild=$need_build) ==="
  BEFORE_SIGS=$(sigs)

  if [ "$need_build" -eq 1 ]; then
    waited=0
    while :; do
      n=$(nbuilds); n=${n:-9}
      [ "$n" -le 1 ] && { echo "  $(stamp) queue clear (other builds=$n) -- taking the slot"; break; }
      sleep 20; waited=$((waited + 20))
      printf '  %s queue: other builds=%s waited=%ss\n' "$(stamp)" "$n" "$waited"
    done
    OLD=$(sha256sum "$EXE" 2>/dev/null | awk '{print $1}'); OLD=${OLD:-none}
    echo "  $(stamp) building (old sha $OLD)..."
    powershell -NoProfile -ExecutionPolicy Bypass -File "$ROOT/scripts/lane-build.ps1" \
      -Lane poppy -Profile perf -NoWait > "$LANE" 2>&1
    grep -E '^(LANE_BUSY_CHECK|REFUSING|BUILD_DONE|EXE_AFTER|GATE|VERDICT)' "$LANE" | sed 's/^/    /'
    if grep -q '^REFUSING' "$LANE"; then
      echo "  refused by the busy gate; the log on disk is an older build's, not this attempt's. sleeping 120s."
      sleep 120
      continue
    fi
    # `grep -c` exits 1 when the count is 0, so `|| echo N` would append a
    # SECOND line and every numeric test after it would blow up on "0\n9".
    # `|| true` keeps grep's own "0".
    errs=$(grep -c '^error' "$LOG" 2>/dev/null || true)
    mine=$(grep -c '^error.*look\.rs' "$LOG" 2>/dev/null || true)
    echo "  $(stamp) errors=${errs:-?} (naming look.rs: ${mine:-?})"
    if [ "${errs:-1}" -ne 0 ]; then
      grep -A2 '^error' "$LOG" | grep -E '^\s+-->' | sort -u | sed 's/^/    /'
      [ "${mine:-0}" -ne 0 ] && { echo "  STOP: look.rs is in the error list -- this lane's bug."; exit 2; }
      echo "  not this lane."
      wait_for_edit "$BEFORE_SIGS"
      continue
    fi
    grep -q '^ *Finished' "$LOG" || { echo "  errors=0 but no 'Finished' -- link never completed."; wait_for_edit "$BEFORE_SIGS"; continue; }
    echo "  $(grep -m1 '^ *Finished' "$LOG" | tr -d '\r')"
    NEW=$(sha256sum "$EXE" | awk '{print $1}')
    [ "$NEW" == "$OLD" ] && { echo "  FAIL relink -- byte-identical exe."; exit 3; }
    echo "  relink PASS  $OLD -> $NEW  mtime $(date -r "$EXE" '+%F %T')"
    need_build=0
  fi

  # --- shoot the A/B: one binary, one lever ------------------------------
  echo "  $(stamp) shooting night-firelit before(ev 7.5) / after(source default)..."
  rm -rf "$OUT"; mkdir -p "$OUT"
  cmd //c "$(cygpath -w "$SHOOT")" "$(cygpath -w "$EXE")" "$(cygpath -w "$OUT")" >/dev/null 2>&1
  shots=$(ls "$OUT"/night-firelit_*.png 2>/dev/null | wc -l)
  echo "  $(stamp) frames on disk: $shots"

  if [ "$shots" -ne 2 ]; then
    echo "  --- why the capture produced no frame ---"
    grep -m2 -A1 'panicked at\|error\[B[0-9]*\]' "$OUT/_shoot.log" 2>/dev/null | sed 's/^/    /'
    grep -ao 'LOOK_FILL gen=[A-Za-z0-9]* ambient=[0-9]* .*ev100=[0-9.]*' "$OUT/_shoot.log" 2>/dev/null \
      | sed 's/.*\(gen=[A-Za-z0-9]*\) \(ambient=[0-9]*\).*\(ev100=[0-9.]*\)/    reached look setup: \1 \2 \3/'
    if wait_for_edit "$BEFORE_SIGS"; then need_build=1; fi
    continue
  fi

  echo
  echo "=== artefact proof: LOOK_FILL ev100 on both legs (runtime, not a strings scan) ==="
  grep -ao 'LOOK_FILL gen=[A-Za-z0-9]* ambient=[0-9]* .*ev100=[0-9.]*' "$OUT/_shoot.log" \
    | sed 's/.*\(gen=[A-Za-z0-9]*\) \(ambient=[0-9]*\).*\(ev100=[0-9.]*\)/  \1 \2 \3/' | uniq -c
  ls -l "$OUT"/*.png | sed 's/^/  /'
  echo
  python scripts/_poppy_ev100_night_gate.py "$OUT" "Hour::NIGHT.ev100 7.5 -> 8.6"
  rc=$?
  echo "GATE EXIT=$rc"
  echo "CHAIN DONE $(stamp)"
  exit 0
done

echo "gave up after $MAXTRY attempts."
exit 1
