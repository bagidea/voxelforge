#!/usr/bin/env python3
"""One TSV row per swept frame, parsed out of the REAL grader's stdout.

Deliberately a parser, not a re-implementation: `grade_gate.py` is the locked
gate (`L >= 55` etc). Re-deriving the numbers here would let a sweep report a
PASS the actual gate would refuse. So this shells the grader and reads it.

Usage: _flamingo_g6_row.py <frame-nohud2.png> <label> <env-string> <out.tsv>

EXIT CODE (added 2026-08-14): being a parser is only half of not lying. This
script wrote `G3=? G5=? G6=?` into the TSV and exited 0 when the grader had
crashed and printed nothing at all -- so a sweep that graded NOTHING looked
exactly like a sweep that passed. `grade_gate.py` already carries its own
verdict in its exit code; this now carries it too:

    0 = grader answered and all three measurable gates are P
    1 = grader answered and at least one gate is F
    2 = grader did not answer (no `MEASURABLE GATES:` line) -- the row is still
        written, tagged UNREADABLE, but the run is not green

The TSV row is always appended, so a red exit never costs the sweep its record.
"""
import re
import subprocess
import sys
from pathlib import Path

frame, label, envs, tsv = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
here = Path(__file__).parent

proc = subprocess.run(
    [sys.executable, str(here / "grade_gate.py"), frame],
    capture_output=True, text=True,
)
out = proc.stdout

def grab(pattern, cast=float, default=""):
    m = re.search(pattern, out)
    return cast(m.group(1)) if m else default

p05 = grab(r"interior p05-L=([\d.]+)%")
min_gb = grab(r"min\(G,B\)=(\d+)", int)
spread = grab(r"spread=([\d.]+)")
g6_rgb = grab(r"golden patch @\([\d,]+\) RGB=\(([\d,]+)\)", str)
g6_l = grab(r"golden patch .*? L=([\d.]+)")

verdict = re.search(r"MEASURABLE GATES: G3=(\w)\s+G5=(\w)\s+G6=(\w)", out)
g3, g5, g6 = verdict.groups() if verdict else ("?", "?", "?")

row = f"{label}\t{envs}\t{p05}\t{min_gb}\t{spread}\t{g6_rgb}\t{g6_l}\t{g3}\t{g5}\t{g6}"
if not verdict:
    row += "\tUNREADABLE"
with open(tsv, "a", encoding="utf-8") as fh:
    fh.write(row + "\n")
print(row)

if not verdict:
    print(f"! grade_gate.py printed no verdict for {frame} "
          f"(exit={proc.returncode}); row tagged UNREADABLE -> exit 2",
          file=sys.stderr)
    if proc.stderr.strip():
        print(proc.stderr.rstrip(), file=sys.stderr)
    raise SystemExit(2)

failed = [n for n, v in (("G3", g3), ("G5", g5), ("G6", g6)) if v != "P"]
if failed:
    print(f"-> {label}: {', '.join(failed)} FAIL per grade_gate.py -> exit 1")
    raise SystemExit(1)
print(f"-> {label}: G3/G5/G6 all P -> exit 0")
