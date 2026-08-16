#!/usr/bin/env bash
# _pixel_ai_ab.sh — enemy-AI lane: build once, shoot BOTH sides of the A/B.
#
# One binary, two runs, one difference: VOXELFORGE_AI_LEGACY. That is on
# purpose. A separate "before" build would differ in link stamps, dep versions
# and whatever else drifted in the tree that day, so a diff between two builds
# cannot isolate the AI change (docs/LANES.md; and the office already got
# burned assuming a commit clock dates a binary). The lever keeps every other
# byte identical.
#
# Chain: build -> run legacy -> run tactics -> plot -> gate.
# Detached-friendly: everything goes to $OUT/, with a DONE stamp at the end, so
# no caller ever has to sit on the process.
#
# Build goes into target-combat via --target-dir, exactly like
# scripts/prove_combat.sh — never the shared target/, which the integration
# lead owns.
set -uo pipefail
export PATH="/usr/bin:/bin:$PATH"
cd "$(dirname "$0")/.."

OUT=${OUT:-_pixel_ai_ab}
BIN=${BIN:-./target-combat/debug/voxelforge.exe}
RUN_TIMEOUT=${RUN_TIMEOUT:-240}
mkdir -p "$OUT"
rm -f "$OUT/DONE" "$OUT/FAILED"

say() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$OUT/chain.log"; }

say "=== enemy-AI A/B chain start ==="
say "bin=$BIN out=$OUT"

# ── 1. build ─────────────────────────────────────────────────────────────
if [ "${SKIP_BUILD:-0}" != "1" ]; then
  say "--- build (cargo build --bin voxelforge --target-dir target-combat) ---"
  bash scripts/build_safe.sh build --bin voxelforge --target-dir target-combat \
    >>"$OUT/build.log" 2>&1
  rc=$?
  say "build exit=$rc"
  if [ "$rc" -ne 0 ]; then
    say "BUILD FAILED — last 40 lines:"
    tail -40 "$OUT/build.log" | tee -a "$OUT/chain.log"
    echo "build rc=$rc" > "$OUT/FAILED"
    exit 1
  fi
fi

if [ ! -x "$BIN" ]; then
  say "NO BINARY at $BIN"
  echo "no binary" > "$OUT/FAILED"
  exit 1
fi
# Grade the artifact, not the exit code alone: a build can return 0 having
# skipped the link (docs/LANES.md "Grade the exit code, never the tail").
say "binary: $(stat -c '%y  %s bytes' "$BIN")"

# ── 2. the two runs ──────────────────────────────────────────────────────
# Identical env except VOXELFORGE_AI_LEGACY. Shot dirs are separated so the
# second run cannot silently overwrite the first one's stills.
run_side() {
  local side="$1" legacy="$2"
  say "--- run: $side ---"
  rm -rf "$OUT/$side-shots"
  mkdir -p "$OUT/$side-shots"
  (
    export VOXELFORGE_PLAY=1
    export VOXELFORGE_AI_DEMO=1
    export VOXELFORGE_AI_TRACE="$OUT/$side.csv"
    export VOXELFORGE_AI_SHOTS="$OUT/$side-shots"
    [ "$legacy" = "1" ] && export VOXELFORGE_AI_LEGACY=1
    timeout "$RUN_TIMEOUT" "$BIN"
  ) >"$OUT/$side.log" 2>&1
  local rc=$?
  echo "exit=$rc" >> "$OUT/$side.log"
  say "$side exit=$rc  trace_rows=$( [ -f "$OUT/$side.csv" ] && wc -l < "$OUT/$side.csv" || echo 0 )  shots=$(ls "$OUT/$side-shots" 2>/dev/null | wc -l)"
  grep -E '^AI_MODE|^AI_DEMO state|^AI_DEMO overall|^AI_TRACE' "$OUT/$side.log" | tee -a "$OUT/chain.log"
  return $rc
}

run_side before 1
run_side after 0

# ── 3. plot ──────────────────────────────────────────────────────────────
say "--- plot ---"
python scripts/_pixel_ai_plot.py "$OUT/before.csv" "$OUT/after.csv" \
  "$OUT/ai-before-after.png" >>"$OUT/chain.log" 2>&1
prc=$?
say "plot exit=$prc"
tail -6 "$OUT/chain.log"

# ── 4. gate ──────────────────────────────────────────────────────────────
# The claim this lane is closing on is "enemies advance-retreat-evade-gang up,
# they don't walk straight in". These gates are the numeric form of that, and
# each one is read out of the AFTER run's own log — never assumed.
fails=0
gate() { # gate <name> <ok:0|1> <detail>
  if [ "$2" = "0" ]; then echo "  PASS $1 — $3" | tee -a "$OUT/chain.log"
  else echo "  FAIL $1 — $3" | tee -a "$OUT/chain.log"; fails=$((fails+1)); fi
}

A="$OUT/after.log"; B="$OUT/before.log"
for s in alert reposition "lunge telegraph" "lunge dash"; do
  if grep -q "AI_DEMO state $s: PASS" "$A"; then gate "after:$s" 0 "reached"
  else gate "after:$s" 1 "never reached in the tactics run"; fi
done
# The lever must actually change something — a legacy run that also reaches the
# new states would mean the gate is not testing the lever at all.
if grep -q "AI_DEMO state alert: FAIL" "$B"; then
  gate "lever" 0 "legacy run never reaches alert (the lever applied)"
else
  gate "lever" 1 "legacy run ALSO reached alert — lever did not apply"
fi
[ -s "$OUT/ai-before-after.png" ] && gate "panel" 0 "$OUT/ai-before-after.png" \
  || gate "panel" 1 "panel not written"

say "=== chain done: $fails gate failure(s) ==="
if [ "$fails" -eq 0 ]; then echo "ok" > "$OUT/DONE"; else echo "gates=$fails" > "$OUT/DONE"; fi
exit "$fails"
