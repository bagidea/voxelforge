#!/usr/bin/env python3
"""One TSV row per swept G7 frame — the locked gates AND the new axes, together.

Deliberately a PARSER over the three real graders, not a re-implementation. A
sweep that re-derived `L >= 55` or the vegetation targets here could report a
PASS the actual gate would refuse, which is the one failure mode that makes a
whole sweep worthless. So it shells `grade_gate.py` (G3/G5/G6), `grade_axes.py`
(p95 + micro-contrast) and `grade_g7.py` (vegetation) and reads their stdout.

Both halves matter on every row: the G7 levers (post-saturation, white balance,
haze) all move the SAME pixels G3/G5/G6 grade, so a row that fixes the greens
while quietly dropping G6 under its sunlit floor is a regression, not a win, and
has to be visible as one in the same table.

Usage: _flamingo_g7_row.py <frame-nohud2.png> <label> <env-string> <out.tsv>

EXIT CODE (added 2026-08-14): parsing the real grader is only half of not lying.
This wrote `G3=? G5=? G6=?` and a vegetation `F` into the TSV and exited 0 when
a grader had crashed and printed nothing -- a row that graded NOTHING was
indistinguishable, to `$?`, from a row that passed. Both graders carry a verdict
in their own exit code; this now carries the combined one:

    0 = both graders answered, G3/G5/G6 all P and grade_g7's summary is PASS
    1 = both answered and at least one verdict is a real FAIL
    2 = a grader did not answer (no `MEASURABLE GATES:` line, or no `=>` summary
        from grade_g7) -- the row is still written, tagged UNREADABLE, but the
        run is not green

The TSV row is always appended, so a red exit never costs the sweep its record.
"""
import re
import subprocess
import sys
from pathlib import Path

frame, label, envs, tsv = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
here = Path(__file__).parent


def run(script, *args):
    return subprocess.run(
        [sys.executable, str(here / script), *args],
        capture_output=True, text=True,
    )


p_gate = run("grade_gate.py", frame)
p_axes = run("grade_axes.py", "--profile", "gameplay", frame)
p_g7 = run("grade_g7.py", "--frame", frame)
gate, axes, g7 = p_gate.stdout, p_axes.stdout, p_g7.stdout


def grab(out, pattern, cast=float, default=""):
    m = re.search(pattern, out)
    if not m:
        return default
    try:
        return cast(m.group(1))
    except ValueError:
        return default


p05 = grab(gate, r"interior p05-L=([\d.]+)%")
spread = grab(gate, r"spread=([\d.]+)")
g6_l = grab(gate, r"golden patch .*? L=([\d.]+)")
m = re.search(r"golden patch @\([\d,]+\) RGB=\((\d+),(\d+),(\d+)\)", gate)
g6_rb = (int(m.group(1)) - int(m.group(3))) if m else ""

v = re.search(r"MEASURABLE GATES: G3=(\w)\s+G5=(\w)\s+G6=(\w)", gate)
g3, g5, g6 = v.groups() if v else ("?", "?", "?")

p95 = grab(axes, r"highlight p95\s+([\d.]+)")
micro = grab(axes, r"micro-contrast\s+([\d.]+)")

veg_s = grab(g7, r"saturation\s+([\d.]+)%\s+need")
veg_h = grab(g7, r"hue\s+([\d.]+)deg need")
# grade_g7 prints one `=> PASS|FAIL` summary line. No summary = it never got far
# enough to have an opinion, which is NOT the same as a vegetation FAIL.
veg_sum = re.search(r"=>\s+(PASS|FAIL)", g7)
veg_v = ("P" if re.search(r"->\s+PASS\s+\[", g7) else "F") if veg_sum else "?"

row = "\t".join(str(x) for x in
                [label, envs, p05, spread, g6_rb, g6_l, g3, g5, g6, p95, micro, veg_s, veg_h, veg_v])
unreadable = [name for name, ok in (("grade_gate.py", v), ("grade_g7.py", veg_sum)) if not ok]
if unreadable:
    row += "\tUNREADABLE"
with open(tsv, "a", encoding="utf-8") as fh:
    fh.write(row + "\n")
print(row)

if unreadable:
    print(f"! no verdict from {', '.join(unreadable)} for {frame} "
          f"(exits: gate={p_gate.returncode} axes={p_axes.returncode} "
          f"g7={p_g7.returncode}); row tagged UNREADABLE -> exit 2", file=sys.stderr)
    for p in (p_gate, p_axes, p_g7):
        if p.stderr.strip():
            print(p.stderr.rstrip(), file=sys.stderr)
    raise SystemExit(2)

failed = [n for n, val in (("G3", g3), ("G5", g5), ("G6", g6), ("vegetation", veg_v))
          if val != "P"]
if failed:
    print(f"-> {label}: {', '.join(failed)} FAIL per the real graders -> exit 1")
    raise SystemExit(1)
print(f"-> {label}: G3/G5/G6 + vegetation all PASS -> exit 0")
