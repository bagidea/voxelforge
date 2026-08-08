#!/usr/bin/env bash
# Poppy — grading guard.
#
# Every gate number in this repo is only meaningful on a de-HUDded frame: the
# `[E]` prompt glyphs are ~250 and sit on the ground, so a raw `--play` capture
# reads "window blown out" (G5) and drags the p95 axis up to the UI. The rule was
# already written down; it was enforced by eye, and eyes are how the last round
# graded a raw frame from a stale exe. So it is enforced here instead: anything
# that is not a `*-nohud2.png` that exists on disk is refused before a single
# number is printed.
#
# Usage: _poppy_grade_nohud2.sh <frame-nohud2.png> ...
set -uo pipefail
cd "$(dirname "$0")/.."

[[ $# -gt 0 ]] || { echo "usage: $0 <frame-nohud2.png> ..."; exit 2; }

bad=0
for f in "$@"; do
  case "$f" in
    *-nohud2.png) ;;
    *) echo "REFUSED  not de-HUDded (need *-nohud2.png): $f"; bad=1; continue;;
  esac
  [[ -f "$f" ]] || { echo "REFUSED  missing on disk: $f"; bad=1; }
done
[[ $bad -eq 0 ]] || { echo; echo "nothing graded."; exit 2; }

for f in "$@"; do
  echo "################################################################"
  echo "# $f   ($(stat -c '%y' "$f" | cut -d. -f1))"
  echo "################################################################"
  python scripts/grade_gate.py "$f"
  python scripts/grade_axes.py --profile gameplay "$f"
  echo
done
