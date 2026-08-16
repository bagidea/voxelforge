#!/usr/bin/env python3
"""Score v6 sweep candidates on the SAME three clauses the v5 gate grades.

One row per candidate frame, one column per clause, and the bars are the ones
`_poppy_lookv5_gate.py` enforces -- read off the scene's committed `_before.png`
so a candidate that looks good here cannot look different there:

  floor   p05 >= 20.40                     (absolute)
  warm    R-B over [p35,p75] >= before     (relative to the v2 plate)
  sep     spread >= before                 (relative to the v2 plate)

Estimators are IMPORTED from `_poppy_lookv3_pairs.py`, never re-typed. A second
copy of `warmth` that drifts is how a search ends up optimising something the
gate does not measure.

Usage: python scripts/_poppy_lookv6_measure.py <scene> <sweepdir> [beforedir]
"""
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np

_pairs = __import__("_poppy_lookv3_pairs")
load, luma, warmth, orient = _pairs.load, _pairs.luma, _pairs.warmth, _pairs.orient

P05_FLOOR = 20.40

scene = sys.argv[1]
sweep = pathlib.Path(sys.argv[2])
before_dir = pathlib.Path(sys.argv[3] if len(sys.argv) > 3 else "docs/assets/look")

b = load(before_dir / f"{scene}_before.png")
lb = luma(b)
p05b, wb, ob = float(np.percentile(lb, 5)), warmth(b, lb), orient(b, lb)

print(f"scene {scene}   before: p05 {p05b:.2f}  warm {wb:.2f}  spread {ob:.2f}")
print(f"{'candidate':22s} {'p05':>7s} {'floor':>5s}  {'warm':>7s} {'d':>7s} {'warm':>5s}  "
      f"{'spread':>7s} {'d':>6s} {'sep':>4s}  {'all':>4s}")

for p in sorted(sweep.glob(f"{scene}__*.png")):
    label = p.name[len(scene) + 2:-4]
    a = load(p)
    la = luma(a)
    p05a, wa, oa = float(np.percentile(la, 5)), warmth(a, la), orient(a, la)
    c_floor, c_warm, c_sep = p05a >= P05_FLOOR, wa >= wb, oa >= ob
    m = lambda c: "PASS" if c else "FAIL"
    print(f"{label:22s} {p05a:7.2f} {m(c_floor):>5s}  {wa:7.2f} {wa - wb:+7.2f} {m(c_warm):>5s}  "
          f"{oa:7.2f} {oa - ob:+6.2f} {m(c_sep):>4s}  "
          f"{m(c_floor and c_warm and c_sep):>4s}")
