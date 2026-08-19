#!/usr/bin/env python3
"""Poppy — PBR/post-stack before/after gate, on Kevin's pinned metric definitions.

WHY A DRIVER AND NOT AN EDIT TO KEVIN'S SCRIPT. `_kevin_art_gap_measure2.py`
hard-codes the four plates its report is about; pointing it at my plates would
silently rewrite `_kevin_art_gap.json` — the file his report cites. So this
imports the SAME `measure()` (identical Sobel / luma-zone / hue definitions, no
re-derivation) and only changes which files go in and where the JSON comes out.

Targets for the render-path lift (outdoor scene):
    edge_mean >= 45     detail per unit area — flatness is the thing being fixed
    shadow    <= 0.45   fraction of pixels with Rec.601 luma < 85
    highlight >= 0.20   fraction with luma > 170

`shadow`/`highlight` come out of `measure()` as FRACTIONS 0..1; the brief quotes
them as percents (45 / 20). Compared as fractions here, printed both ways, so a
reader cannot mistake a 0.45 for a 45.

Usage:  _poppy_pbr_measure.py <label>=<path> [<label>=<path> ...]
"""
import os
import sys
import json

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# (key, comparison, threshold) — threshold in the units measure() returns.
GATES = [
    ("edge_mean", ">=", 45.0),
    ("shadow", "<=", 0.45),
    ("highlight", ">=", 0.20),
]
KEYS = ["edge_mean", "shadow", "midtone", "highlight", "lum_mean", "strong",
        "sat_mean", "sat_std", "hue90", "occupied", "warmcool"]


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    rows = {}
    for arg in argv:
        if "=" not in arg:
            print("bad arg (want label=path): %s" % arg)
            return 2
        label, path = arg.split("=", 1)
        if not os.path.isfile(path):
            print("MISSING %-14s %s" % (label, path))
            return 2
        r = measure(path, label)
        r["path"] = path
        r["bytes"] = os.path.getsize(path)
        rows[label] = r

    print("%-12s %s" % ("metric", "  ".join("%12s" % n for n in rows)))
    for k in KEYS:
        print("%-12s %s" % (k, "  ".join("%12.3f" % rows[n][k] for n in rows)))
    print("%-12s %s" % ("w x h", "  ".join(
        "%12s" % ("%dx%d" % (rows[n]["w"], rows[n]["h"])) for n in rows)))

    print("\n=== GATES ===")
    verdicts = {}
    for label in rows:
        ok = True
        for key, cmp_, thr in GATES:
            v = rows[label][key]
            passed = (v >= thr) if cmp_ == ">=" else (v <= thr)
            ok &= passed
            shown = v * 100 if key in ("shadow", "highlight") else v
            thr_shown = thr * 100 if key in ("shadow", "highlight") else thr
            print("  %-8s %-10s %8.2f %s %6.2f  %s"
                  % (label, key, shown, cmp_, thr_shown, "PASS" if passed else "FAIL"))
        verdicts[label] = ok
        print("  %-8s => %s\n" % (label, "ALL PASS" if ok else "FAIL"))

    out = os.path.join(ROOT, "_poppy_pbr_metrics.json")
    with open(out, "w") as f:
        json.dump({n: dict({k: rows[n][k] for k in KEYS},
                           path=rows[n]["path"], bytes=rows[n]["bytes"],
                           w=rows[n]["w"], h=rows[n]["h"],
                           all_pass=verdicts[n]) for n in rows}, f, indent=2)
    print("WROTE", out)
    return 0 if all(verdicts.values()) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
