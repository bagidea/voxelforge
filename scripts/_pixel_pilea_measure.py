#!/usr/bin/env python3
"""Pixel — Pile A (grade/lighting) before/after scorer, art-gap 2026-08-17.

This scores ARBITRARY plates against the 9 Pile A targets. It does NOT reimplement
the metrics: it imports `measure` and `hue_sat_val` from `_kevin_art_gap_measure`,
the same two functions `_kevin_art_gap_measure2.py` calls, and repeats that script's
warm/cool block verbatim. A metric pipeline that is re-typed is a metric pipeline
that drifts (memory: "metric pipelines don't travel"), so the only safe wrapper is
one that calls the original.

That claim is CHECKED, not asserted. `--control` re-measures the two plates the
canonical script publishes numbers for and fails loudly if any key disagrees past
1e-9 — run it before believing any after-number this file prints.

Usage:
  python scripts/_pixel_pilea_measure.py --control
  python scripts/_pixel_pilea_measure.py before=<png> after=<png> [more=<png> ...]
"""
import os

os.environ["LOKY_MAX_CPU_COUNT"] = "4"
import sys
import json

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure, hue_sat_val  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The 9 Pile A targets, verbatim from docs/art-gap-vs-reference-2026-08-17.md
# targets 1-4 (target 5, edge detail, is Pile B and is NOT gated here -- it is
# reported so a grade change that quietly destroyed detail cannot hide).
#   key, human name, comparator, bound, the interior "before" the brief quotes
TARGETS = [
    ("hue90",     "hue spread 90% (deg)", ">=", 90.0,  24.0),
    ("occupied",  "occupied hue bins/36", ">=", 15.0,   5.0),
    ("cool_pct",  "cool share %",         ">=", 12.0,   0.0),
    ("warmcool",  "warm:cool ratio",      "<=",  6.0,   float("inf")),
    ("highlight", "highlight % (>170)",   ">=", 20.0,   8.7),
    ("shadow",    "shadow % (<85)",       "<=", 45.0,  60.3),
    ("lum_mean",  "mean luma",            ">=", 105.0, 79.5),
    ("sat_std",   "saturation std",       ">=", 0.22,  0.173),
    ("sat_mean",  "saturation mean",      "<=", 0.75,  0.863),
]
# Percent-denominated keys `measure` returns as fractions 0-1. The canonical
# script prints them as fractions and the report's table as percents; the targets
# above are written in the REPORT's units, so convert here rather than editing
# either side's meaning.
AS_PCT = {"highlight", "shadow", "midtone", "strong"}

REPORT_KEYS = ["hue90", "occupied", "sat_mean", "sat_std", "shadow", "midtone",
               "highlight", "lum_mean", "edge_mean", "strong", "warmcool",
               "warm_pct", "cool_pct", "neutral_pct"]


def score(path):
    """Every number the canonical script publishes, for one plate."""
    r = measure(path, os.path.basename(path))
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    h, s, _ = hue_sat_val(a)
    # --- verbatim from _kevin_art_gap_measure2.py lines 53-58 ---------------
    sat_mask = s >= 0.08
    warm = ((h < 70) | (h >= 340)) & sat_mask
    cool = (h >= 170) & (h < 270) & sat_mask
    tot = int(sat_mask.sum())
    r["warm_pct"] = 100.0 * warm.sum() / tot
    r["cool_pct"] = 100.0 * cool.sum() / tot
    r["neutral_pct"] = 100.0 * (sat_mask & ~warm & ~cool).sum() / tot
    # -----------------------------------------------------------------------
    out = {k: float(r[k]) for k in REPORT_KEYS}
    for k in AS_PCT:
        out[k] *= 100.0
    return out


def control():
    """Prove this file reproduces the canonical script, before it is believed.

    `_kevin_art_gap.json` is written by `_kevin_art_gap_measure2.py`. Re-measuring
    its plates here must land on the same floats -- if it does not, this wrapper is
    measuring something else and every after-number below it is worthless.
    """
    ref = os.path.join(ROOT, "_kevin_art_gap.json")
    if not os.path.exists(ref):
        print("CONTROL SKIP: no _kevin_art_gap.json -- run _kevin_art_gap_measure2.py first")
        return 2
    canon = json.load(open(ref))
    plates = {
        "beauty": os.path.join(ROOT, r"_fl_beauty_20260816\beauty-wide-nohud2.png"),
        "g4": os.path.join(ROOT, r"_fl_g2g4_20260816\g4-only-nohud2.png"),
        "outdoor_noon": os.path.join(ROOT, r"_fl_lookv4\outdoor-noon-after-nohud2.png"),
    }
    bad = 0
    for name, p in plates.items():
        mine = score(p)
        for k in REPORT_KEYS:
            theirs = canon[name][k]
            if k in AS_PCT:
                theirs *= 100.0
            m = mine[k]
            same = (not np.isfinite(theirs) and not np.isfinite(m)) or \
                   (np.isfinite(theirs) and abs(theirs - m) <= 1e-9)
            if not same:
                print("CONTROL MISMATCH %s.%s canonical=%r wrapper=%r" % (name, k, theirs, m))
                bad += 1
    print("CONTROL %s -- %d plates x %d keys, %d mismatches"
          % ("PASS" if bad == 0 else "FAIL", len(plates), len(REPORT_KEYS), bad))
    return 0 if bad == 0 else 1


def fmt(v):
    return "inf" if not np.isfinite(v) else ("%.3f" % v)


def main(argv):
    if "--control" in argv:
        return control()
    plates = []
    for a in argv:
        if "=" not in a:
            print("bad arg %r -- want label=path" % a)
            return 2
        label, p = a.split("=", 1)
        if not os.path.isabs(p):
            p = os.path.join(ROOT, p)
        if not os.path.exists(p):
            print("MISSING %s -> %s" % (label, p))
            return 2
        plates.append((label, p, score(p)))
    if not plates:
        print(__doc__)
        return 2

    w = 14
    print("%-24s %s" % ("metric", "  ".join("%*s" % (w, l) for l, _, _ in plates)))
    for k in REPORT_KEYS:
        print("%-24s %s" % (k, "  ".join("%*s" % (w, fmt(r[k])) for _, _, r in plates)))

    # --- the gate table -----------------------------------------------------
    last_label, _, last = plates[-1]
    print("\nPILE A GATES (graded on the LAST plate: %s)" % last_label)
    print("%-24s %8s %8s %10s  %s" % ("target", "brief", "now", "bound", "verdict"))
    npass = 0
    for key, name, cmp_, bound, before in TARGETS:
        v = last[key]
        ok = (v >= bound) if cmp_ == ">=" else (v <= bound)
        # inf can only satisfy a >= bound; a <= bound against inf is a FAIL, which
        # is the honest reading of warm:cool with zero cool pixels.
        if not np.isfinite(v):
            ok = cmp_ == ">="
        npass += ok
        print("%-24s %8s %8s %10s  %s"
              % (name, fmt(before), fmt(v), "%s %g" % (cmp_, bound), "PASS" if ok else "FAIL"))
    print("\n%d/%d PASS" % (npass, len(TARGETS)))
    print("(Pile B, reported not gated: edge_mean=%s strong=%s)"
          % (fmt(last["edge_mean"]), fmt(last["strong"])))
    return 0 if npass == len(TARGETS) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
