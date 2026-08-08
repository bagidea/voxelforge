#!/usr/bin/env python3
"""One TSV row per swept frame, parsed out of the REAL grader's stdout.

Deliberately a parser, not a re-implementation: `grade_gate.py` is the locked
gate (`L >= 55` etc). Re-deriving the numbers here would let a sweep report a
PASS the actual gate would refuse. So this shells the grader and reads it.

Usage: _flamingo_g6_row.py <frame-nohud2.png> <label> <env-string> <out.tsv>
"""
import re
import subprocess
import sys
from pathlib import Path

frame, label, envs, tsv = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
here = Path(__file__).parent

out = subprocess.run(
    [sys.executable, str(here / "grade_gate.py"), frame],
    capture_output=True, text=True,
).stdout

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
with open(tsv, "a", encoding="utf-8") as fh:
    fh.write(row + "\n")
print(row)
