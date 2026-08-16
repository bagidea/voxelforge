#!/usr/bin/env bash
# Poppy -- Hour::NIGHT.ev100 7.5 -> 8.6: queue -> build -> verdict -> relink
# proof -> shoot the night A/B -> grade it. Detached and unattended on purpose.
#
# THE REASON THIS RETRIES INSTEAD OF BUILDING ONCE. The shared worktree does not
# compile right now, and NOT because of this lane: the 08:13 log failed on
# `client/src/audio.rs` (E0308, a `let Some(..) = <Result>` binding) and
# `client/src/main.rs` (E0382, `cfg` used after `.insert_resource(cfg)` moved it).
# Both belong to other lanes and this lane may not touch them. So the chain
# builds, and if the only errors are outside look.rs it says so, waits for one of
# those files to be edited, and tries again -- rather than burning a slot in a
# loop or, worse, "fixing" someone else's file.
#
# THE VERDICT IS `grep -c '^error'` ON THE FULL CARGO LOG. Not the tail (cargo
# keeps printing `Compiling` after a failure), not a pipe's exit code. Same rule
# as scripts/_poppy_lookv7_chain.sh; see lane-build.ps1's header for why.
#
# QUEUEING RULE: start only when at most ONE other cargo build is live, so this
# lane is the second and never the third. The third concurrent build is what
# takes the box out with 0xC0000142 STATUS_DLL_INIT_FAILED -- that is a commit-
# headroom failure, not a code failure, and `-j 1` does not help because the peak
# comes from the number of builds, not the number of jobs.
#
# Usage: bash scripts/_poppy_ev100_night_chain.sh
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
LOG=$ROOT/_poppy_build.log
LANE=$ROOT/_poppy_ev100night_lane.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OUT=$ROOT/_poppy_ev100night
SHOOT=$ROOT/scripts/_poppy_ev100_night_shoot.cmd
WATCH=(client/src/audio.rs client/src/main.rs client/src/look.rs)
MAXTRY=10

# Live cargo BUILD launches on the box. cargo shows up as TWO processes per
# launch -- the rustup shim (`cargo  build ...`, no path) and the real toolchain
# exe (`"...\.rustup\toolchains\...\bin\cargo.exe" build ...`) -- so counting
# cargo.exe doubles every build. Counting unique CreationDate does NOT fix it
# either: Win32_Process timestamps carry sub-second precision and the pair
# differs in the fraction, which is exactly how the first run of this chain sat
# at "other builds=2" against a box that had ONE build on it. Matching the real
# toolchain path counts each launch once.
#
# `tasklist` alone cannot do any of this -- it shows cargo.exe with no way to
# tell whose it is.
nbuilds() {
  powershell -NoProfile -Command \
    "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\" | Where-Object { \$_.CommandLine -like '*\\bin\\cargo.exe*' }).Count" \
    2>/dev/null | tr -d '\r' | tr -dc '0-9'
}

stamp() { date '+%H:%M:%S'; }
sigs()  { for f in "${WATCH[@]}"; do printf '%s:%s ' "$f" "$(date -r "$f" +%s 2>/dev/null || echo 0)"; done; }

echo "=== ev100 night chain started $(stamp) ==="
echo "watching for other lanes to unbreak: ${WATCH[*]}"

for try in $(seq 1 $MAXTRY); do
  echo
  echo "=== attempt $try/$MAXTRY ==="

  # --- [1] queue behind at most one other build --------------------------
  waited=0
  while :; do
    n=$(nbuilds); n=${n:-9}
    [ "$n" -le 1 ] && { echo "  $(stamp) queue clear (other builds=$n) -- taking the slot"; break; }
    sleep 20; waited=$((waited + 20))
    printf '  %s queue: other builds=%s waited=%ss\n' "$(stamp)" "$n" "$waited"
    [ "$waited" -ge 1800 ] && { echo "  $(stamp) queue wait capped at 1800s -- going anyway is NOT allowed; retrying"; break; }
  done
  n=$(nbuilds); [ "${n:-9}" -le 1 ] || { echo "  still $n builds live, not stacking a third; sleeping 120s"; sleep 120; continue; }

  # --- [2] record the exe we are about to replace ------------------------
  OLD=$(sha256sum "$EXE" 2>/dev/null | awk '{print $1}'); OLD=${OLD:-none}
  echo "  old exe sha $OLD"
  BEFORE_SIGS=$(sigs)

  # --- [3] build (lane-build owns the busy gate + the raw redirect) -------
  echo "  $(stamp) building..."
  powershell -NoProfile -ExecutionPolicy Bypass -File "$ROOT/scripts/lane-build.ps1" \
    -Lane poppy -Profile perf -NoWait > "$LANE" 2>&1
  grep -E '^(LANE_BUSY_CHECK|REFUSING|BUILD_DONE|EXE_AFTER|GATE|VERDICT)' "$LANE" | sed 's/^/    /'

  if grep -q '^REFUSING' "$LANE"; then
    # The log on disk belongs to an EARLIER build; grepping it now would grade
    # somebody else's compile as this attempt's verdict.
    echo "  refused by the busy gate -- target-poppy is in use. sleeping 120s, not counting this try."
    sleep 120
    continue
  fi

  # --- [4] verdict: grep -c '^error' on the full log, nothing else --------
  errs=$(grep -c '^error' "$LOG" 2>/dev/null || echo 9)
  mine=$(grep -c "^error.*look\.rs" "$LOG" 2>/dev/null || echo 0)
  echo "  $(stamp) errors=$errs (naming look.rs: $mine)"
  if [ "${errs:-9}" -ne 0 ]; then
    echo "  --- error sites ---"
    grep -A2 '^error' "$LOG" | grep -E '^\s+-->' | sort -u | sed 's/^/    /'
    if [ "${mine:-0}" -ne 0 ]; then
      echo "  STOP: look.rs is in the error list. That is this lane's bug -- fix it, do not retry."
      exit 2
    fi
    echo "  not this lane. waiting for another lane to edit one of the watched files..."
    for _ in $(seq 1 90); do   # 90 x 20s = 30 min, then retry regardless
      sleep 20
      [ "$(sigs)" != "$BEFORE_SIGS" ] && { echo "  $(stamp) a watched file moved -- retrying"; break; }
    done
    continue
  fi
  if ! grep -q '^ *Finished' "$LOG"; then
    echo "  errors=0 but no 'Finished' line -- the link never completed. Not shooting."
    continue
  fi
  echo "  $(grep -m1 '^ *Finished' "$LOG" | tr -d '\r')"

  # --- [5] relink proof --------------------------------------------------
  NEW=$(sha256sum "$EXE" | awk '{print $1}')
  echo "  new exe sha $NEW"
  if [ "$NEW" == "$OLD" ]; then
    echo "  FAIL -- byte-identical to the pre-build exe. The link did not happen."
    exit 3
  fi
  echo "  relink PASS  mtime $(date -r "$EXE" '+%F %T')"
  printf '%s  %s\n' "$OLD" "$NEW" > "$ROOT/_poppy_ev100night_relink.txt"

  # --- [6] shoot the A/B, one binary, one lever --------------------------
  echo
  echo "=== shooting night-firelit before(ev 7.5) / after(source default) ==="
  mkdir -p "$OUT"
  cmd //c "$(cygpath -w "$SHOOT")" "$(cygpath -w "$EXE")" "$(cygpath -w "$OUT")" >/dev/null 2>&1
  if [ ! -f "$OUT/_shoot.done" ]; then
    echo "  no _shoot.done -- the capture died. tail of the shoot log:"
    tail -20 "$OUT/_shoot.log" 2>/dev/null | sed 's/^/    /'
    exit 4
  fi
  ls -l "$OUT"/*.png | sed 's/^/  /'

  # The frame's OWN log has to say ev100=8.60 on the no-lever leg, or the exe
  # that shot it was not built from this source. A `strings` scan cannot tell
  # the two exes apart -- both carry the token.
  echo
  echo "=== artefact proof: LOOK_FILL ev100 on both legs ==="
  grep -ao 'LOOK_FILL gen=[A-Za-z0-9]* ambient=[0-9]* .*ev100=[0-9.]*' "$OUT/_shoot.log" \
    | sed 's/.*\(gen=[A-Za-z0-9]*\) \(ambient=[0-9]*\).*\(ev100=[0-9.]*\)/  \1 \2 \3/' \
    | uniq -c

  # --- [7] grade ---------------------------------------------------------
  echo
  python scripts/_poppy_ev100_night_gate.py "$OUT" "Hour::NIGHT.ev100 7.5 -> 8.6"
  echo "GATE EXIT=$?"
  echo "CHAIN DONE $(stamp)"
  exit 0
done

echo "gave up after $MAXTRY attempts -- the tree never compiled."
exit 1
