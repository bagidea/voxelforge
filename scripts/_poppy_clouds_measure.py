#!/usr/bin/env python3
"""Poppy -- A3 cloud deck: the A/B number, against its own null floor.

Same reduction the MAT_MAPS round was signed off on, so the two are comparable:

  mean|dRGB|   mean over pixels of the mean absolute per-channel difference
  px>=8        share of pixels whose mean absolute difference clears 8 levels

and every one of those is reported for THREE pairs per scene:

  A/B    before_<scene>.png  vs  after_<scene>.png     (lever off vs on)
  floor  after_<scene>.png   vs  null_<scene>.png      (same config, twice)
  cross  before_<scene>.png  vs  null_<scene>.png      (the A/B, other null)

The floor is not decoration. Nothing in this engine is bit-stable across two
runs -- streaming order, TAA history and frame timing all move -- so an A/B
number is only evidence if it stands well clear of the difference between two
shots of the SAME config. The ratio A/B : floor is the verdict; the raw number
is not.

`sky` columns repeat the same reductions over the pixels the deck can actually
reach. The mask is NOT derived from the difference image -- a diff-derived mask
makes "the change landed on target" true by construction. It is the region ABOVE
the horizon line, found on the BEFORE plate alone: the topmost run of rows whose
row-median luminance is above the frame median. A cloud deck that only moves
ground pixels would be a bug, and this column is what would show it.

Usage:
  _poppy_clouds_measure.py <dir> <scene> [<scene> ...]
"""
import os
import sys

import numpy as np
from PIL import Image


def load(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)


def reduce_pair(a, b, mask=None):
    d = np.abs(a - b).mean(axis=2)
    if mask is not None:
        d = d[mask]
    return float(d.mean()), float((d >= 8.0).mean() * 100.0)


def sky_mask(before):
    """Rows above the horizon, from the BEFORE plate only.

    Luminance per row; the sky is the leading block of rows brighter than the
    frame's own median. Deliberately crude and deliberately independent of the
    after plate -- its only job is to be a region the clouds could plausibly
    occupy that was NOT chosen by looking at where they landed.
    """
    lum = before @ np.array([0.2126, 0.7152, 0.0722])
    rows = np.median(lum, axis=1)
    thr = np.median(lum)
    m = np.zeros(before.shape[:2], dtype=bool)
    for y, v in enumerate(rows):
        if v <= thr:
            break
        m[y, :] = True
    return m


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    root, scenes = argv[1], argv[2:]

    print(f"{'scene':<16} {'pair':<7} {'mean|dRGB|':>11} {'px>=8 %':>9} "
          f"{'sky mean':>9} {'sky px>=8 %':>12}")
    print("-" * 70)
    rc = 0
    for scene in scenes:
        paths = {t: os.path.join(root, f"{t}_{scene}.png") for t in ("before", "after", "null")}
        missing = [p for p in paths.values() if not os.path.exists(p)]
        if missing:
            print(f"{scene:<16} MISSING {missing}")
            rc = 1
            continue
        img = {t: load(p) for t, p in paths.items()}
        shapes = {t: a.shape for t, a in img.items()}
        if len(set(shapes.values())) != 1:
            print(f"{scene:<16} SHAPE MISMATCH {shapes}")
            rc = 1
            continue
        m = sky_mask(img["before"])

        rows = [
            ("A/B", "before", "after"),
            ("floor", "after", "null"),
            ("cross", "before", "null"),
        ]
        vals = {}
        for label, x, y in rows:
            full = reduce_pair(img[x], img[y])
            sky = reduce_pair(img[x], img[y], m) if m.any() else (float("nan"),) * 2
            vals[label] = full
            print(f"{scene:<16} {label:<7} {full[0]:>11.3f} {full[1]:>9.2f} "
                  f"{sky[0]:>9.3f} {sky[1]:>12.2f}")
        ratio = vals["A/B"][0] / vals["floor"][0] if vals["floor"][0] > 0 else float("inf")
        print(f"{scene:<16} {'ratio':<7} {ratio:>11.1f}x  (A/B mean over floor mean)  "
              f"sky rows {int(m[:, 0].sum())}/{m.shape[0]}")
        print("-" * 70)
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv))
