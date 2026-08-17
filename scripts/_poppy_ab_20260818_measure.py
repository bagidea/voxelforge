#!/usr/bin/env python3
"""Poppy -- the three numbers the brief asked for, on Kevin's pinned definitions.

The brief wants edge density, mean hue and %cool pixel for each plate, compared
against the reference behind `docs/art-gap-vs-reference-2026-08-17.png`.

Two of those three are NOT in Kevin's `measure()` return:

  * `measure()` gives `hue90` (the minimal arc holding 90% of hue mass -- a
    SPREAD), not a mean hue.
  * it gives `warmcool` (a warm/cool mass RATIO), not a cool percentage.

So this driver imports Kevin's module and reuses its `rgb` / `hue_sat_val`
loaders and its `measure()` verbatim for `edge_mean` -- no metric is
re-derived here -- and adds only the two missing reductions, built on the SAME
masks his report pins:

  saturated pixel   sat >= 0.08
  cool hue          170 <= hue < 270

`cool_pct_sat` (of saturated pixels) is the primary number, because that is the
population Kevin's warm/cool figures are quoted over; `cool_pct_all` (of every
pixel) is printed beside it so a reader cannot mistake one for the other.

Mean hue is a CIRCULAR mean (atan2 of the summed unit vectors). An arithmetic
mean of an angle is meaningless -- red at 355 deg and red at 5 deg would average
to cyan at 180.

`clip_pct` is carried because a saturation/warmth reading can be manufactured by
a channel pinned to 0 or 255; if that number moves between two plates, the
colour numbers beside it are not comparable.

Usage:  _poppy_ab_20260818_measure.py <label>=<path> [<label>=<path> ...]
"""
import os
import sys
import json

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402
from _kevin_art_gap_measure import measure, rgb, hue_sat_val  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SAT_MIN = 0.08          # Kevin's saturated-pixel threshold
COOL_LO, COOL_HI = 170.0, 270.0   # Kevin's cool hue band


def extras(path):
    """Circular mean hue, cool share and clip share -- Kevin's masks, exactly."""
    a = rgb(Image.open(path))
    h, s, _v = hue_sat_val(a)

    sat_mask = s >= SAT_MIN
    hs = h[sat_mask]
    if hs.size:
        rad = np.deg2rad(hs.astype(np.float64))
        ang = np.arctan2(np.sin(rad).sum(), np.cos(rad).sum())
        hue_mean = float(np.rad2deg(ang) % 360.0)
        # resultant length: 0 = hue mass evenly spread (mean is meaningless),
        # 1 = all one hue. Printed so a mean hue is never read without it.
        hue_r = float(np.hypot(np.sin(rad).sum(), np.cos(rad).sum()) / hs.size)
    else:
        hue_mean, hue_r = float("nan"), 0.0

    cool = (h >= COOL_LO) & (h < COOL_HI) & sat_mask
    total = float(h.size)
    cool_pct_all = 100.0 * float(cool.sum()) / total
    cool_pct_sat = (100.0 * float(cool.sum()) / float(sat_mask.sum())
                    if sat_mask.sum() else 0.0)

    clipped = (a <= 0).any(axis=2) | (a >= 255).any(axis=2)
    clip_pct = 100.0 * float(clipped.sum()) / total

    return {"hue_mean": hue_mean, "hue_r": hue_r,
            "cool_pct_sat": cool_pct_sat, "cool_pct_all": cool_pct_all,
            "sat_pct": 100.0 * float(sat_mask.sum()) / total,
            "clip_pct": clip_pct}


ROWS = [
    ("edge_mean",    "edge density (Sobel mean)", "%12.2f"),
    ("hue_mean",     "mean hue (circular, deg)",  "%12.1f"),
    ("hue_r",        "  hue concentration 0-1",   "%12.3f"),
    ("cool_pct_sat", "cool px % (of saturated)",  "%12.2f"),
    ("cool_pct_all", "cool px % (of all px)",     "%12.2f"),
    ("sat_pct",      "  saturated px %",          "%12.2f"),
    ("clip_pct",     "  clipped px %",            "%12.2f"),
    ("strong",       "  strong-edge frac",        "%12.3f"),
    ("lum_mean",     "  luma mean",               "%12.1f"),
]


def main(argv):
    if not argv:
        print(__doc__)
        return 2

    rows = {}
    order = []
    for arg in argv:
        if "=" not in arg:
            print("bad arg (want label=path): %s" % arg)
            return 2
        label, path = arg.split("=", 1)
        if not os.path.isfile(path):
            print("MISSING %-16s %s" % (label, path))
            return 2
        r = measure(path, label)
        r.update(extras(path))
        r["path"] = path
        rows[label] = r
        order.append(label)

    w = max(len(n) for n in order) if order else 8
    print("%-28s %s" % ("metric", "  ".join("%12s" % n for n in order)))
    print("-" * (28 + 14 * len(order)))
    for key, title, fmt in ROWS:
        print("%-28s %s" % (title, "  ".join(fmt % rows[n][key] for n in order)))
    print("%-28s %s" % ("w x h", "  ".join(
        "%12s" % ("%dx%d" % (rows[n]["w"], rows[n]["h"])) for n in order)))

    # --- deltas vs the reference, when one is present --------------------------
    if "REF" in rows:
        print("\n=== gap to REF (positive = REF is higher) ===")
        for key, title, _fmt in ROWS[:5]:
            print("%-28s %s" % (title, "  ".join(
                "%12.2f" % (rows["REF"][key] - rows[n][key])
                for n in order if n != "REF")))
        print("(columns: %s)" % ", ".join(n for n in order if n != "REF"))

    out = os.path.join(ROOT, "_poppy_ab_20260818_metrics.json")
    with open(out, "w") as f:
        json.dump({n: {k: rows[n][k] for k in
                       [r[0] for r in ROWS] + ["w", "h", "path"]}
                   for n in order}, f, indent=2)
    print("\nWROTE", out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
