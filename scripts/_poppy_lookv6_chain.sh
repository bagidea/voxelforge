#!/usr/bin/env bash
# Poppy — v6: wait for build → verdict by `grep -c '^error'` → gate → shoot →
# measure → GRADE. Same shape as `_poppy_lookv5_chain.sh`, with two changes.
#
# 1. THE ARTEFACT PROOF IS STILL A HASH INEQUALITY, and it has to be. v6 moves
#    five numeric constants and adds two (`TEMPERATURE_V3`,
#    `AMBIENT_COLOR_V3_NIGHT`) — no new string literals, so
#    `_poppy_lookv3_gate.sh`'s .rdata markers PASS on the stale v5 exe just as
#    happily as on a fresh one. `_poppy_lookv6_oldexe.sha256` was written BEFORE
#    the build could overwrite the file (207ce021…, the exe that shot the v5
#    plates); if the new hash equals it, the link never happened and the pair
#    would be a v5 pair with a v6 caption.
#
# 2. THE GATE RUNS INSIDE THE CHAIN, NOT AFTER IT IN A WRITE-UP. v5's separation
#    regression on `evening-raking` existed in the shipped plates and was only
#    found when the gate was run by hand for the doc, a day late. Step [6/6] is
#    that same gate, on the frames just shot, with its exit code kept.
#
# Usage: bash scripts/_poppy_lookv6_chain.sh [outdir]
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT=$PWD
DONE=$ROOT/_poppy_lookv6_build.done
LOG=$ROOT/_poppy_lookv6_build.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OLD_SHA=$(awk '{print $1}' "$ROOT/_poppy_lookv6_oldexe.sha256" 2>/dev/null || echo none)
OUT="${1:-$ROOT/docs/assets/look}"

echo "=== [1/6] waiting for $(basename "$DONE") ==="
waited=0
while [[ ! -f "$DONE" ]]; do
  sleep 15
  waited=$((waited + 15))
  printf '  %4ds  log=%s lines  target touched=%s\n' \
    "$waited" "$(wc -l < "$LOG" 2>/dev/null || echo 0)" \
    "$(date -r "$ROOT/target-poppy" +%H:%M:%S 2>/dev/null || echo '-')"
done
echo "  $(cat "$DONE")"

# The whole verdict. `tail` is not the answer: cargo prints `Compiling` lines
# AFTER a failure, so the last line of a broken build is routinely green.
errs=$(grep -c '^error' "$LOG" || true)
mine=$(grep -c '^error.*look\.rs' "$LOG" || true)
echo "=== [2/6] errors: $errs total ($mine naming look.rs) ==="
if [[ "$errs" -ne 0 ]]; then
  grep -B1 -A6 '^error' "$LOG" | head -60
  echo "  no usable exe — stopping before the gate."
  exit 1
fi
# `Finished` is the other half: zero errors on a log that never linked is what a
# killed build looks like (0xc0000142 takes the build script out, not rustc).
if ! grep -q '^ *Finished' "$LOG"; then
  echo "  errors=0 but no 'Finished' line — the link did not complete. Not shooting."
  exit 1
fi
echo "  $(grep -m1 '^ *Finished' "$LOG")"

echo "=== [3/6] gate ==="
bash scripts/_poppy_lookv3_gate.sh "$EXE" || exit 1
new_sha=$(sha256sum "$EXE" | awk '{print $1}')
echo "GATE6: old sha $OLD_SHA"
echo "GATE6: new sha $new_sha"
if [[ "$new_sha" == "$OLD_SHA" ]]; then
  echo "GATE6: FAIL — byte-identical to the v5 exe. The link did not happen;"
  echo "GATE6:        shooting now would caption a v5 pair as v6."
  exit 1
fi
echo "GATE6: PASS — this is a different binary from the one that shot the v5 plates."

echo "=== [4/6] shoot -> $OUT ==="
mkdir -p "$OUT"
cmd //c "$(cygpath -w "$ROOT/scripts/_poppy_lookv3_shoot.cmd")" "$(cygpath -w "$EXE")" "$(cygpath -w "$OUT")"

shopt -s nullglob
shot=("$OUT"/*_before.png "$OUT"/*_after.png)
echo "  ${#shot[@]} frames on disk"
[[ ${#shot[@]} -eq 6 ]] || { echo "  EXPECTED 6 — capture incomplete, not grading it."; exit 1; }

# The runtime half of the artefact proof. v6 leaves both rim levels alone
# (600/20) — the v6 tell is the night ambient, which `LOOK_FILL` prints: v5 is
# `ambient=14`, v6 is `ambient=26`.
echo "  LOOK_FILL ambient values seen this run:"
grep -ao 'gen=V3 ambient=[0-9]*' "$OUT/_shoot.log" | sort | uniq -c | sed 's/^/    /'

echo "=== [5/6] measure ==="
python scripts/_poppy_lookv3_pairs.py "$OUT"

echo "=== [6/6] gate: the three clauses, as an exit code ==="
python scripts/_poppy_lookv5_gate.py "$OUT"
echo "GATE EXIT=$?"
