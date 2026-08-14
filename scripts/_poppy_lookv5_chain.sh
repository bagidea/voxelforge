#!/usr/bin/env bash
# Poppy — v5: wait for build → verdict by `grep -c '^error'` → gate → shoot →
# measure. Same shape as `_poppy_lookv4_chain.sh`; two things are different and
# both are scars.
#
# 1. THE GATE IS NO LONGER ENOUGH ON ITS OWN. `_poppy_lookv3_gate.sh` proves an
#    exe carries the v3 RIG by grepping two string literals out of .rdata. Every
#    exe from v4 onward carries those too, so on this round it would PASS on the
#    stale binary and the pair would be v4-vs-v4 with a v5 caption. The v5 change
#    is six numeric constants and a `ColorGrading` literal — no new strings to
#    grep — so the artefact check here is a SHA-256 INEQUALITY against the v4
#    exe's own hash, recorded before the build could overwrite it
#    (`_poppy_lookv5_oldexe.sha256`, 59a76ae7…). A commit clock never dates a
#    binary; a hash does.
#
# 2. THE RUNTIME CONFIRMS THE SPECIFIC NUMBERS. `apply_fill_rig` prints one
#    `LOOK_FILL … rim=<lux>@<deg>` line per run. v4's night rim is 30 lux, v5's
#    is 20 ([`RIM_LUX_NIGHT`]), so the shoot log itself says which binary drew
#    the night plate. Checked after the capture, not before, because it needs a
#    run to exist.
#
# Usage: bash scripts/_poppy_lookv5_chain.sh [outdir]
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT=$PWD
DONE=$ROOT/_poppy_lookv5_build.done
LOG=$ROOT/_poppy_lookv5_build.log
EXE=$ROOT/target-poppy/perf/voxelforge.exe
OLD_SHA=$(awk '{print $1}' "$ROOT/_poppy_lookv5_oldexe.sha256" 2>/dev/null || echo none)
OUT="${1:-$ROOT/docs/assets/look}"

echo "=== [1/5] waiting for $(basename "$DONE") ==="
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
echo "=== [2/5] errors: $errs total ($mine naming look.rs) ==="
if [[ "$errs" -ne 0 ]]; then
  grep -B1 -A6 '^error' "$LOG" | head -60
  echo "  no usable exe — stopping before the gate."
  exit 1
fi

echo "=== [3/5] gate ==="
bash scripts/_poppy_lookv3_gate.sh "$EXE" || exit 1
new_sha=$(sha256sum "$EXE" | awk '{print $1}')
echo "GATE5: old sha $OLD_SHA"
echo "GATE5: new sha $new_sha"
if [[ "$new_sha" == "$OLD_SHA" ]]; then
  echo "GATE5: FAIL — byte-identical to the v4 exe. The link did not happen;"
  echo "GATE5:        shooting now would caption a v4 pair as v5."
  exit 1
fi
echo "GATE5: PASS — this is a different binary from the one that shot the v4 plates."

echo "=== [4/5] shoot -> $OUT ==="
mkdir -p "$OUT"
cmd //c "$(cygpath -w "$ROOT/scripts/_poppy_lookv3_shoot.cmd")" "$(cygpath -w "$EXE")" "$(cygpath -w "$OUT")"

shopt -s nullglob
shot=("$OUT"/*_before.png "$OUT"/*_after.png)
echo "  ${#shot[@]} frames on disk"
[[ ${#shot[@]} -eq 6 ]] || { echo "  EXPECTED 6 — capture incomplete, not grading it."; exit 1; }

# The runtime half of the artefact proof (see the header). v4 night = 30 lux.
echo "  LOOK_FILL rim values seen this run:"
grep -ao 'rim=[0-9]*lux@[0-9]*deg' "$OUT/_shoot.log" | sort | uniq -c | sed 's/^/    /'

echo "=== [5/5] measure ==="
python scripts/_poppy_lookv3_pairs.py "$OUT"
