#!/usr/bin/env python3
"""Whole-frame scan for the clasp's own hue band (#4FC9D6, teal).

Same method as docs/VERDICT-a6-teal-accent-2026-08-10.md's repro: window
150-220 deg hue, delta(max-min channel)>15, max channel>30 (loose enough to
catch a desaturated/darkened render of the same hue, tight enough to exclude
near-black/near-grey noise). Verified against the literal Color::srgb
constant in anim.rs (currently #4FC9D6, HSV hue ~185.8 deg).

Usage: python scripts/_flamingo_clasp_hue_scan.py <frame.png> ...
"""
import colorsys
import sys

import numpy as np
from PIL import Image

HUE_LO, HUE_HI = 150.0, 220.0
DELTA_MIN = 15
MAX_MIN = 30

CONST = (0.310, 0.788, 0.839)
h, s, v = colorsys.rgb_to_hsv(*CONST)
print(f"reference constant srgb{CONST} -> hue {h*360:.1f} deg (sanity check)")


def scan(path: str):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im, dtype=np.float64)
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    mx = a.max(axis=2)
    mn = a.min(axis=2)
    delta = mx - mn

    # vectorized hue calc matching colorsys.rgb_to_hsv semantics
    hue = np.zeros_like(mx)
    nz = delta > 0
    is_r = nz & (mx == r)
    is_g = nz & (mx == g) & ~is_r
    is_b = nz & (mx == b) & ~is_r & ~is_g
    hue[is_r] = (60 * ((g[is_r] - b[is_r]) / delta[is_r]) + 360) % 360
    hue[is_g] = (60 * ((b[is_g] - r[is_g]) / delta[is_g]) + 120) % 360
    hue[is_b] = (60 * ((r[is_b] - g[is_b]) / delta[is_b]) + 240) % 360

    mask = (hue >= HUE_LO) & (hue <= HUE_HI) & (delta > DELTA_MIN) & (mx > MAX_MIN)
    n = int(mask.sum())
    total = mx.size
    print(f"{path}  ({a.shape[1]}x{a.shape[0]}, {total} px): {n} matching px")
    if n:
        ys, xs = np.where(mask)
        print(f"  bbox: x[{xs.min()}-{xs.max()}] y[{ys.min()}-{ys.max()}]  centroid=({xs.mean():.0f},{ys.mean():.0f})")
    return n


if __name__ == "__main__":
    for p in sys.argv[1:]:
        scan(p)
