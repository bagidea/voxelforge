#!/usr/bin/env bash
# A6.2 close-out chain — run AFTER a build of HEAD (8f09601's clasp z is in
# `client/src/anim.rs` but has never been compiled).
#
# This script does NOT build. `gate3_shoot.sh` refuses to shoot a binary older
# than the newest `client/src/**/*.rs`, so a stale exe fails loudly here rather
# than producing frames that silently grade the previous fix.
#
# What it proves, in order:
#   1. the three gate3 frames shoot clean (boot / walk / combat)
#   2. HUD is stripped before anything is graded  (rubric §3 — no exceptions)
#   3. the character mask + C-axis card exists for the boot frame
#   4. the reserved cool accent #4FC9D6 actually RENDERS  <- the A6.2 defect
#   5. the retired `hue split` number, printed as advisory only
#   6. the rubric regrade: grade_gate.py (the authority for G3/G5/G6) + grade_axes
#
# Usage:  bash scripts/_flamingo_a62_chain.sh [OUTDIR]
set -euo pipefail

OUT="${1:-docs/assets/gate3-a6-teal-2026-08-14}"
CAM="0,-14.32,5.289"           # the solved boom every prior A6 shoot used
mkdir -p "$OUT"

echo "=============================================================="
echo "[1/6] gate3 shoot -> $OUT   (freshness guard is ON — a stale exe stops here)"
echo "=============================================================="
OUT="$OUT" bash scripts/gate3_shoot.sh

echo
echo "=============================================================="
echo "[2/6] de-HUD (rubric step 3 — grade the -nohud2 files, never the raw --play frame)"
echo "=============================================================="
for t in boot walk combat; do
  python scripts/_flamingo_dehud2.py "$OUT/gate3-after-$t.png"
done

echo
echo "=============================================================="
echo "[3/6] character mask + C axes (boot = the idle pose A6 grades)"
echo "     WATCH C11 material clusters: it reads 5.00 against the concept's 6.00 while"
echo "     the gem is occluded. Monanisa's design note predicted the gem is the 6th"
echo "     cluster (housing reuses \`trim\`, adds none). C11 5 -> 6 is an INDEPENDENT"
echo "     corroboration of the fix that does not come from my own accent scanner."
echo "=============================================================="
python scripts/grade_character.py "$OUT/gate3-after-boot-nohud2.png" \
  --cam "$CAM" --ref-json _fl_char/ref-auren-hero.json \
  --json "$OUT/boot-char.json" || true

echo
echo "=============================================================="
echo "[4/6] A6.2 THE DEFECT: does #4FC9D6 render?   (pre-fix control = 0 px x3)"
echo "     The boot frame carries the HARD verdict (--gate) because step [3] wrote it a"
echo "     charmask; walk/combat have none and will read UNRELIABLE by design — a"
echo "     whole-frame scan can be fooled by the scene's own teal (rubric Pass 9b)."
echo "=============================================================="
A62_GATE=0
python scripts/_flamingo_a62_accent_presence.py \
  "$OUT"/gate3-after-{boot,walk,combat}-nohud2.png \
  --json "$OUT/a62-accent.json" || A62_GATE=1
# Deliberately NOT under `set -e`: a FAIL here is a result to report, not a reason
# to skip the rubric regrade below. The verdict is re-printed at the end.
python scripts/_flamingo_a62_accent_presence.py \
  "$OUT/gate3-after-boot-nohud2.png" --gate || A62_GATE=1

echo
echo "=============================================================="
echo "[5/6] ADVISORY ONLY — the retired hue-split number"
echo "     (control: the 3 approved concept sheets score 1.4-5.8 deg on this)"
echo "=============================================================="
python scripts/_flamingo_a62_huesep_control.py "$OUT/gate3-after-boot-nohud2.png" \
  --charmask "$OUT/gate3-after-boot-nohud2-charmask.png" \
  --label "gate3 boot (post-fix)" || true

echo
echo "=============================================================="
echo "[6/6] rubric regrade — grade_gate.py is the authority for G3/G5/G6"
echo "=============================================================="
for t in boot walk combat; do
  echo "--- $t ---"
  python scripts/grade_gate.py "$OUT/gate3-after-$t-nohud2.png" || true
done
echo "--- P0 axes, --profile gameplay (DOF fg:bg is framing-dependent and is NOT a"
echo "    verdict on play frames — rubric \"DOF trap\" note; warmth/blue/sat ADVISORY per rubric:83) ---"
python scripts/grade_axes.py --profile gameplay "$OUT"/gate3-after-{boot,walk,combat}-nohud2.png || true

echo
echo "=============================================================="
if [ "$A62_GATE" = "0" ]; then
  echo "A6.2' accent gate: PASS — the reserved cool accent renders and reads"
else
  echo "A6.2' accent gate: FAIL — accent does not render (or no charmask to judge on)"
  echo "  Before moving the clasp a FOURTH time, re-run the instrument control on THIS"
  echo "  plate class — the 4-px read floor is denominated in pixels, so a control that"
  echo "  passed at 2560x1360 does not carry to a smaller plate:"
  echo "    python scripts/_flamingo_a62_synth_control.py $OUT/gate3-after-boot-nohud2.png \\"
  echo "           --x <mapped-x> --y <mapped-y> --size <projected-gem-px>"
fi
echo "chain complete -> $OUT"
exit "$A62_GATE"
