#!/usr/bin/env bash
# Sun — AFTER scorecard: triggered when _rose_recap_20260806/DONE appears.
# Runs regrade on before→after pairs, updates the scorecard, picks best pair.
set -uo pipefail
cd "$(dirname "$0")/.."

RECAP="_rose_recap_20260806"
SCORECARD="docs/aaa-gap-scorecard-2026-08-06.md"
REGRADE_JSON="$RECAP/regrade_after.json"
PAIR_DIR="$RECAP/_pairs"

say() { echo "$(date '+%H:%M:%S') sun: $*"; }

# ---- 0. verify DONE and frames ------------------------------------------------
DONE_STATUS=$(cat "$RECAP/DONE" 2>/dev/null || echo "MISSING")
say "DONE status: $DONE_STATUS"

if [ "$DONE_STATUS" != "FINISH_OK" ]; then
  say "ABORT: finisher did not complete successfully (DONE=$DONE_STATUS)"
  say "Check $RECAP/finish.log and $RECAP/capture.log"
  echo "SCORECARD_NOT_UPDATED: finisher status=$DONE_STATUS" > "$RECAP/_sun_status.txt"
  exit 1
fi

# Count after frames
N_AFTER=$(find "$RECAP" -maxdepth 1 -name '*-nohud2.png' | wc -l)
say "after frames found: $N_AFTER"
if [ "$N_AFTER" -lt 4 ]; then
  say "ABORT: not enough after frames ($N_AFTER)"
  exit 1
fi

# ---- 1. build paired directories ----------------------------------------------
say "building paired directories..."
rm -rf "$PAIR_DIR"
mkdir -p "$PAIR_DIR/before" "$PAIR_DIR/after"

# BEFORE frames → copy with normalized keys
# gate3-after-boot  ← gate3-boot-new
# gate3-after-combat ← gate3-combat-new
# gate3-after-walk  ← gate3-walk-new
# grade-vista-2026-08-05 ← grade-vista-new
declare -A MAP=(
  ["gate3-after-boot"]="gate3-boot-new"
  ["gate3-after-combat"]="gate3-combat-new"
  ["gate3-after-walk"]="gate3-walk-new"
  ["grade-vista-2026-08-05"]="grade-vista-new"
)

PAIRS=0
for before_key in "${!MAP[@]}"; do
  after_key="${MAP[$before_key]}"
  common_key="${after_key%-new}"   # gate3-boot, gate3-combat, gate3-walk, grade-vista

  # Find before frame
  before_src=$(find docs/assets -name "${before_key}*nohud2.png" -print -quit 2>/dev/null)
  if [ -z "$before_src" ]; then
    say "  SKIP $common_key: no before frame for key $before_key"
    continue
  fi

  # After frame
  after_src="$RECAP/${after_key}-nohud2.png"
  if [ ! -f "$after_src" ]; then
    say "  SKIP $common_key: no after frame $after_src"
    continue
  fi

  cp "$before_src" "$PAIR_DIR/before/${common_key}-nohud2.png"
  cp "$after_src"  "$PAIR_DIR/after/${common_key}-nohud2.png"
  say "  pair: $common_key  ←  $before_src  /  $after_src"
  PAIRS=$((PAIRS + 1))
done

if [ "$PAIRS" -eq 0 ]; then
  say "ABORT: no matching pairs found"
  exit 1
fi
say "$PAIRS pairs ready"

# ---- 2. run regrade -----------------------------------------------------------
say "running regrade.py..."
python scripts/regrade.py \
  --before "$PAIR_DIR/before/" \
  --after  "$PAIR_DIR/after/" \
  --json "$REGRADE_JSON" \
  > "$RECAP/regrade_console.txt" 2>&1
RC=$?
say "regrade exit=$RC"

# ---- 3. update scorecard ------------------------------------------------------
say "updating scorecard..."
python scripts/_sun_update_scorecard.py \
  --json "$REGRADE_JSON" \
  --scorecard "$SCORECARD" \
  --out "$RECAP/aaa-gap-scorecard-after-2026-08-06.md" \
  --recap-dir "$RECAP" \
  > "$RECAP/scorecard_update.log" 2>&1
say "scorecard update exit=$?"

# ---- 4. pick best visual pair -------------------------------------------------
say "picking best visual pair..."
BEST=$(python scripts/_sun_pick_best_pair.py \
  --json "$REGRADE_JSON" \
  --pair-dir "$PAIR_DIR" \
  --recap-dir "$RECAP" \
  2>&1)
say "best pair: $BEST"

# ---- 5. done ------------------------------------------------------------------
echo "SCORECARD_UPDATED=$PAIRS pairs" > "$RECAP/_sun_status.txt"
say "ALL DONE"
say "  scorecard: $RECAP/aaa-gap-scorecard-after-2026-08-06.md"
say "  regrade json: $REGRADE_JSON"
say "  best pair: $BEST"
