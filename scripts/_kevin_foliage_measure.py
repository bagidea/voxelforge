#!/usr/bin/env python3
"""Kevin — foliage density pass: measure edge/strong on the before/after stills.

Reuses the pinned `measure()` from _kevin_art_gap_measure.py (Sobel gradient on
Rec.601 luma, gaussian sigma=0.8; `strong` = fraction of px with Sobel > 40).
Prints the RELATIVE before->after delta only — absolute gating vs the shipped
36.8/50.0 plate is invalid here because the 14-Aug target-kevin/perf exe predates
the renderer commits that plate was graded on.

Usage:  python scripts/_kevin_foliage_measure.py [before.png] [after.png]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main():
    before = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "_kevin_foliage", "before.png")
    after = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "_kevin_foliage", "after.png")
    b = measure(before, "before")
    a = measure(after, "after")

    # Delta-only (relative on one binary) — do NOT gate vs the absolute 36.8/50.0
    # plate here: those targets are pinned to a specific renderer commit, and the
    # 14-Aug target-kevin/perf exe predates it. Report the relative change only.
    print("%-12s %12s %12s %12s" % ("metric", "before", "after", "delta"))
    print("%-12s %12.2f %12.2f %+12.2f" % (
        "edge_mean", b["edge_mean"], a["edge_mean"], a["edge_mean"] - b["edge_mean"]))
    print("%-12s %12.4f %12.4f %+12.4f" % (
        "strong", b["strong"], a["strong"], a["strong"] - b["strong"]))

    print("\nstrong %% : before=%.1f%%  after=%.1f%%  delta=%+.1f pp" % (
        b["strong"] * 100, a["strong"] * 100, (a["strong"] - b["strong"]) * 100))
    print("edge_mean delta = %+.2f   strong delta = %+.4f (%+.1f pp)" % (
        a["edge_mean"] - b["edge_mean"],
        a["strong"] - b["strong"],
        (a["strong"] - b["strong"]) * 100,
    ))


if __name__ == "__main__":
    main()
