#!/usr/bin/env python3
"""The three things v5 promised, as a pass/fail — not as a paragraph.

The brief was one sentence with three clauses joined by AND: "rim brighter AND
the shade floor not below 20.40 AND warmth not lower". Three clauses is exactly
the shape that gets reported as two-out-of-three with the third one softened, so
each is a column here and the exit code is their conjunction.

Uses the SAME estimators as `_poppy_lookv3_pairs.py` (imported, not re-typed —
a second copy of `warmth` that drifts is how a gate ends up grading something
nobody measured).

  p05      shade floor, Rec.709 luma. Absolute bar: >= 20.40.
  warmth   R-B over the midtone band [p35, p75]. Relative bar: after >= before.
  spread   median(L >= p75) - median(L <= p25). Relative bar: after >= before.
           Stands in for the RIM clause: the kicker's job is separating a
           surface from what is behind it, and this is the widest separation
           axis available from a still. Deliberately NOT "rim lux went up" —
           a constant is not a frame.

  NAMING, BECAUSE THE FUNCTION IT COMES FROM IS MISLABELLED. It is imported
  from `_poppy_lookv3_pairs.py` as `orient`, whose docstring promises
  top-vs-side face separation found by "the pixels whose 3x3 luma gradient is
  smallest in the vertical screen direction". The body computes no gradient and
  filters nothing (`flat = l.copy()` then two percentiles) — it is a tonal
  SPREAD, not an orientation measure. Imported as-is anyway, and called
  `spread` here, because the v3 and v4 plates were graded with this exact
  function and a corrected estimator would not be comparable to the numbers
  already on record. Flagged, not silently re-implemented; fixing it is its own
  change with its own re-baseline.

Usage: python scripts/_poppy_lookv5_gate.py [dir]
Exit:  0 = all three clauses hold on all three scenes
"""
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np
from PIL import Image

_pairs = __import__("_poppy_lookv3_pairs")
load, luma, warmth, orient = _pairs.load, _pairs.luma, _pairs.warmth, _pairs.orient
SCENES = _pairs.SCENES

P05_FLOOR = 20.40

out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "docs/assets/look")
rows, ok = [], True

print(f"{'scene':16s} {'p05 b':>7s} {'p05 a':>7s} {'floor':>6s}  "
      f"{'warm b':>7s} {'warm a':>7s} {'warm':>5s}  "
      f"{'spread b':>8s} {'spread a':>8s} {'sep':>4s}")

for s in SCENES:
    b, a = load(out / f"{s}_before.png"), load(out / f"{s}_after.png")
    lb, la = luma(b), luma(a)
    p05b, p05a = np.percentile(lb, 5), np.percentile(la, 5)
    wb, wa = warmth(b, lb), warmth(a, la)
    ob, oa = orient(b, lb), orient(a, la)

    c_floor = p05a >= P05_FLOOR
    c_warm = wa >= wb
    c_sep = oa >= ob
    ok &= c_floor and c_warm and c_sep

    m = lambda c: "PASS" if c else "FAIL"
    print(f"{s:16s} {p05b:7.2f} {p05a:7.2f} {m(c_floor):>6s}  "
          f"{wb:7.2f} {wa:7.2f} {m(c_warm):>5s}  "
          f"{ob:8.2f} {oa:8.2f} {m(c_sep):>4s}")

print()
print(f"floor bar: p05(after) >= {P05_FLOOR}   warmth bar: after >= before   "
      f"separation bar: after >= before")
print("V5 GATE:", "PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
