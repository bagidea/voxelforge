#!/usr/bin/env bash
# ===========================================================================
# Poppy — run Flamingo's judge (608eb60/d7f95bd, byte-unmodified) over the
# three 64px pairs, each against ITS OWN null pair.
#
# The judge's control block runs first and is not optional: rubric rule 7 says
# a metric with no control has no cut line, and this whole re-shoot exists
# because the last verdict was measured on a proxy. If the instrument cannot
# reproduce its published numbers today, the verdicts below mean nothing and
# this stops before printing any.
#
# EXIT: 0 all three PASS · 1 at least one FAIL · 2 judge refused / control fail
#
# USAGE
#   scripts/_poppy_matmaps64_judge.sh
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
PUB="docs/assets/look"
OUT="_poppy_matmaps/verdict64"
mkdir -p "$OUT"

stamp() { date "+[%H:%M:%S]"; }

echo "$(stamp) === CONTROL (the judge proving itself before it judges) ==="
python scripts/_fl_matmaps_judge.py control > "$OUT/control.txt" 2>&1
CTL=$?
tail -n 14 "$OUT/control.txt"
echo "$(stamp) control exit=$CTL  (full: $OUT/control.txt)"
if [ "$CTL" -ne 0 ]; then
  echo "REFUSED  the instrument failed its own control — no verdict is issued."
  exit 2
fi
echo

WORST=0
for scene in day evening-raking night-firelit; do
  mkdir -p "$OUT/$scene"
  echo "$(stamp) === JUDGE $scene ==="
  python scripts/_fl_matmaps_judge.py judge \
    "$PUB/matmaps64_${scene}_off.png" \
    "$PUB/matmaps64_${scene}_on.png" \
    --null "$PUB/matmaps64_${scene}_onNULLA.png" "$PUB/matmaps64_${scene}_onNULLB.png" \
    --out "$OUT/$scene" --label-a off --label-b on \
    > "$OUT/$scene/judge.txt" 2>&1
  rc=$?
  # The lines that decide it, plus the two the brief asked about by name.
  grep -E "^  \[(PASS|FAIL|ADV)\]|^VERDICT|BORROWED FLOOR|^REFUSED" "$OUT/$scene/judge.txt" || true
  echo "$(stamp) $scene exit=$rc  (full: $OUT/$scene/judge.txt)"
  [ "$rc" -gt "$WORST" ] && WORST=$rc
  echo
done

echo "$(stamp) === M1 / M4 ROLL-UP (the two the brief asked for) ==="
for scene in day evening-raking night-firelit; do
  printf '  %-16s ' "$scene"
  grep -hE "^  \[(PASS|FAIL)\] M(1|4) " "$OUT/$scene/judge.txt" 2>/dev/null \
    | sed 's/^  //' | tr '\n' '|' || true
  echo
done
echo "$(stamp) worst exit=$WORST"
exit "$WORST"
