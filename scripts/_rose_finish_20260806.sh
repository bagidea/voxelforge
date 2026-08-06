#!/usr/bin/env bash
# Rose — END-TO-END finisher: wait for the target-rose exe the orphaned build is
# still producing, then capture 8 frames (new/ctrl) -> grade -> summary table ->
# compare sheet -> DONE marker. Chained so it runs to completion even if the
# session that launched it is killed (the build already survived one such kill).
#
# WHY IT WAITS ON THE EXE INSTEAD OF BUILDING ITSELF. A build is already running
# into target-rose (started by the previous session's _rose_drive_20260806.sh,
# orphaned but alive — cargo keeps running when its parent bash dies on Windows).
# Starting a second would stack the STATUS_DLL_INIT_FAILED trap. So this script
# treats the running build as the producer and only consumes its artifact.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=_rose_recap_20260806
LOG="$OUT/finish.log"
mkdir -p "$OUT"
: > "$LOG"
say() { echo "$(date '+%H:%M:%S') $*" | tee -a "$LOG"; }

EXE=target-rose/release/voxelforge.exe
SRC=client/src/look.rs

# ---- 1. wait for the exe (built from current source) ------------------------
say "waiting for $EXE newer than $SRC ..."
READY=0
for i in $(seq 1 360); do   # up to 60 min
  if [ -s "$EXE" ] && [ "$EXE" -nt "$SRC" ]; then
    say "exe ready: $(stat -c '%y' "$EXE" | cut -d. -f1)  $(stat -c %s "$EXE") bytes"
    READY=1; break
  fi
  # also bail if a build genuinely died leaving no exe and no cargo running
  [ $((i % 6)) -eq 0 ] && say "  poll#$i  rlibs=$(find target-rose -name '*.rlib' 2>/dev/null | wc -l)  cargo=$(powershell.exe -NoProfile -Command '(Get-CimInstance Win32_Process -Filter "Name=''cargo.exe'' OR Name=''rustc.exe'' OR Name=''link.exe''" | Measure-Object).Count' 2>/dev/null | tr -d '\r ')"
  sleep 10
done
if [ "$READY" -ne 1 ]; then
  say "NO EXE after 60 min — build may have failed. Aborting finisher."
  echo "FINISH_ABORTED_NO_EXE" > "$OUT/DONE"
  exit 1
fi

# ---- 2. capture 8 frames (new/ctrl) from target-rose exe --------------------
say "capturing 8 frames (recap workflow, SRC_EXE=$EXE)..."
SRC_EXE="$EXE" OUT="$OUT" bash scripts/_flamingo_recap_20260806.sh > "$OUT/capture.log" 2>&1
CAP_RC=$?
say "capture exit=$CAP_RC"
tail -6 "$OUT/capture.log" | tee -a "$LOG"
if [ "$CAP_RC" -ne 0 ]; then
  echo "FINISH_ABORTED_CAPTURE=$CAP_RC" > "$OUT/DONE"
  exit 1
fi

# ---- 3. grade (gate + axes + g3 + penumbra per stem×arm) --------------------
say "grading 8 frames..."
OUT="$OUT" bash scripts/_flamingo_recap_grade_20260806.sh > "$OUT/grade.log" 2>&1
say "grade exit=$?  ($(wc -l < "$OUT/grade.log") lines)"

# ---- 4. summary table (parse grade log) -------------------------------------
say "summary table..."
python scripts/_rose_summary.py "$OUT/grade.log" "$OUT/_rose_summary.md" >> "$LOG" 2>&1

# ---- 5. compare sheet (side-by-side before/after images) --------------------
say "compare sheet..."
python scripts/_rose_compare_sheet.py "$OUT" >> "$LOG" 2>&1

echo "FINISH_OK" > "$OUT/DONE"
say "ALL DONE — $OUT/_rose_summary.md  +  $OUT/_compare_sheet_all.png"
