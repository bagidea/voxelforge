#!/usr/bin/env python3
"""Scratch: one compact row per frame for the magenta-fix sweep (Poppy 2026-08-01).

Reuses scripts/colour_gate.py and scripts/grade_axes.py as libraries so the
sweep is graded by the same code the gates run — no second implementation of the
metrics to drift from them.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import colour_gate  # noqa: E402
import grade_axes  # noqa: E402
import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402

HDR = (
    f"{'frame':<18} {'mag%':>6} {'sky R,G,B':>19} {'order':>9} "
    f"{'sunlit R,G,B':>19} {'order':>9} {'warm':>7} {'blue':>6} {'clip%':>6} {'sat':>6}"
)


def main():
    print(HDR)
    print("-" * len(HDR))
    for path in sys.argv[1:]:
        a = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
        lit = a.mean(2) > colour_gate.LIT_MIN_MEAN
        m = (
            lit
            & (a[..., 1] < a[..., 0] - colour_gate.CHANNEL_MARGIN)
            & (a[..., 1] < a[..., 2] - colour_gate.CHANNEL_MARGIN)
        )
        mag = m.sum() / a.shape[0] / a.shape[1] * 100
        sky, _ = colour_gate.sky_patch(a)
        sun = colour_gate.sunlit_patch(a, sky)
        ax = grade_axes.measure(path)

        def trip(v):
            return "  n/a  " if v is None else f"{v[0]:5.1f},{v[1]:5.1f},{v[2]:5.1f}"

        name = os.path.basename(path)[:-4]
        sat_txt = grade_axes.sat_status(ax)[1]   # honest sat, or N-A if clip co-gate fired
        print(
            f"{name:<18} {mag:6.2f} {trip(sky):>19} "
            f"{(colour_gate.order(sky) if sky is not None else '-'):>9} "
            f"{trip(sun):>19} {(colour_gate.order(sun) if sun is not None else '-'):>9} "
            f"{ax['warmth']:7.1f} {ax['blue']:6.1f} {ax['clip']:6.1f} {sat_txt:>6}"
        )
    print("\ntargets:  mag <= 2.00 | sky B > G > R | sunlit R > G > B | "
          "warm >= 110 | blue <= 10 | clip <= 35% | sat >= 90 (honest; N-A if clip > 35%)")


if __name__ == "__main__":
    main()
