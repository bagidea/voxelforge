#!/usr/bin/env python
"""_pixel_bake_diff.py -- pixel-level comparison for the ship-blocker #3 bake.

Prints, for each (a, b) pair given on the command line as a:b ...
  * differing-pixel count + share
  * max / mean absolute channel delta
  * mean RGB of each side

The point is the NOISE FLOOR row. A same-binary, same-env repeat shot tells me what
"identical" actually measures on this box (TAA + a frozen frame should be exact, but
"should be" is not a measurement). Only deltas ABOVE that floor are real drift.
"""
import sys

import numpy as np
from PIL import Image


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.int16)


def row(label, pa, pb):
    a, b = load(pa), load(pb)
    if a.shape != b.shape:
        print(f"{label:<26} SHAPE MISMATCH {a.shape} vs {b.shape}")
        return
    d = np.abs(a - b)
    per_px = d.max(axis=2)
    n = int((per_px > 0).sum())
    total = per_px.size
    print(
        f"{label:<26} diff_px {n:>9,} ({100.0 * n / total:6.3f}%)  "
        f"max {int(d.max()):>3}  mean {d.mean():7.4f}   "
        f"meanRGB_a {a[..., 0].mean():6.1f}/{a[..., 1].mean():5.1f}/{a[..., 2].mean():5.1f}  "
        f"meanRGB_b {b[..., 0].mean():6.1f}/{b[..., 1].mean():5.1f}/{b[..., 2].mean():5.1f}"
    )


if __name__ == "__main__":
    pairs = sys.argv[1:]
    if not pairs:
        print(__doc__)
        sys.exit(2)
    for spec in pairs:
        label, _, rest = spec.partition("=")
        pa, _, pb = rest.partition(":")
        row(label, pa, pb)
