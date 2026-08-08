#!/usr/bin/env python3
"""Collateral check: what the GRADE lever does to pixels axis C never looks at.

grade_g7.py's axis C masks to hue 40..150 (`grade_g7.py:88-98`) — vegetation only.
VOXELFORGE_LOOK_GRADE is a global post-op, so it also moves sky and stone, and a
row can therefore PASS axis C while wrecking the frame. This measures the two
regions the gate is blind to, on the SAME frames the sweep already shot.

Sky = pixels whose hue is in the blue band (180..260) in the REFERENCE frame's
own terms; taking a fixed top-rows box instead would drift as the grade shifts
hue out of the band and silently compare different pixel sets. So the mask is
built ONCE from the reference frame and reused pixel-for-pixel on every frame —
identical framing is guaranteed (camera is pinned on every sweep row).

Usage: _flamingo_g7_sky.py <ref.png> <frame.png> [frame.png ...]
"""
import colorsys
import sys

from PIL import Image


def hsv(p):
    h, s, v = colorsys.rgb_to_hsv(p[0] / 255, p[1] / 255, p[2] / 255)
    return h * 360, s * 100, v * 100


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


ref = Image.open(sys.argv[1]).convert("RGB")
rp, (W, H) = ref.load(), ref.size

sky, stone = [], []
for y in range(0, H, 2):
    for x in range(0, W, 2):
        h, s, v = hsv(rp[x, y])
        if 180 <= h <= 260 and s > 12 and v > 40:
            sky.append((x, y))
        elif (h < 40 or h > 330) and s > 12 and v > 25:
            stone.append((x, y))

print(f"masks from {sys.argv[1]}: sky n={len(sky)}  stone n={len(stone)}\n")
print(f"{'frame':<26} {'sky sat':>8} {'sky hue':>8} {'sky V':>7} | {'stone sat':>9} {'stone hue':>9}")

for path in sys.argv[2:]:
    im = Image.open(path).convert("RGB")
    if im.size != (W, H):
        raise SystemExit(f"! size mismatch {im.size} vs {(W, H)}")
    p = im.load()
    sk = [hsv(p[x, y]) for x, y in sky]
    st = [hsv(p[x, y]) for x, y in stone]
    name = path.split("\\")[-1].replace("-nohud2.png", "")
    print(f"{name:<26} {mean([a[1] for a in sk]):8.1f} {mean([a[0] for a in sk]):8.1f}"
          f" {mean([a[2] for a in sk]):7.1f} | {mean([a[1] for a in st]):9.1f}"
          f" {mean([a[0] for a in st]):9.1f}")
