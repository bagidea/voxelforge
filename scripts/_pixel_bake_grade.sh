#!/usr/bin/env bash
# _pixel_bake_grade.sh -- run the rubric probe over every *-nohud2.png in a dir,
# tee-ing each run (cmd + cwd + stdout/stderr + real exit code) into <dir>/logs/.
# Usage: bash scripts/_pixel_bake_grade.sh <dir> [extra-png ...]
set -u
cd "$(dirname "$0")/.."

DIR="${1:?usage: _pixel_bake_grade.sh <dir> [extra.png ...]}"
shift || true
LOG="$DIR/logs"
mkdir -p "$LOG"

run() {
  local slug="$1"; shift
  local f="$LOG/$slug.log"
  {
    echo "=== $slug"
    echo "when : $(date -Is)"
    echo "cwd  : $(pwd)"
    echo "cmd  : $*"
    echo "--- stdout+stderr ---"
  } > "$f"
  "$@" >> "$f" 2>&1
  local code=$?
  echo "--- exit=$code ---" >> "$f"
  printf '%-40s exit=%s -> %s\n' "$slug" "$code" "$f"
}

for f in "$DIR"/*-nohud2.png "$@"; do
  [[ -f "$f" ]] || continue
  base="$(basename "$f" -nohud2.png)"
  base="$(basename "$base" .png)"
  run "rubric-$base" python scripts/_pixel_rubric_probe.py "$f"
  run "axes-$base"   python scripts/grade_axes.py "$f"
  run "gate-$base"   python scripts/grade_gate.py "$f"
  run "look-$base"   python scripts/grade_look.py "$f"
done
echo "DONE -- logs in $LOG"
