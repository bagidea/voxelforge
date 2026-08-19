#!/usr/bin/env python3
"""Poppy - what actually moved between _matmaps_before.png and _matmaps_after.png.

Two plates, one lever. This prints the numbers I refuse to write a caption
without: how many pixels moved at all, how far, and where the movement is
concentrated. It also drops a side-by-side + amplified-difference sheet so the
Director can see the claim instead of taking it.

The metric is deliberately boring: mean |dRGB| over the frame, the share of
pixels past a visible threshold, and a per-tile heat grid. No fitted estimator,
no mask derived from the difference itself (that would make "the maps did it"
unfalsifiable - see the diff-derived-mask scar).

USAGE  python scripts/_poppy_matmaps_ab_measure.py
"""
import os
import sys

from PIL import Image, ImageChops, ImageDraw
import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BEFORE = os.path.join(ROOT, "_matmaps_before.png")
AFTER = os.path.join(ROOT, "_matmaps_after.png")
SHEET = os.path.join(ROOT, "_matmaps_diff_sheet.png")

for p in (BEFORE, AFTER):
    if not os.path.isfile(p):
        sys.exit("MISSING %s" % p)

b = Image.open(BEFORE).convert("RGB")
a = Image.open(AFTER).convert("RGB")
if b.size != a.size:
    sys.exit("SIZE MISMATCH %s vs %s" % (b.size, a.size))

nb = np.asarray(b, dtype=np.int16)
na = np.asarray(a, dtype=np.int16)
d = np.abs(na - nb)
dmax = d.max(axis=2)          # per-pixel worst channel
npx = dmax.size

print("plates      : %dx%d" % b.size)
print("mean |dRGB| : %.3f  (0 = the lever changed nothing)" % d.mean())
print("max  |dRGB| : %d" % d.max())
for t in (1, 2, 4, 8, 16, 32):
    share = (dmax >= t).mean() * 100.0
    print("  px with |d|>=%-3d : %6.2f%%" % (t, share))

# per-tile heat: where is the movement, not just how much
GX, GY = 8, 4
h, w = dmax.shape
print("per-tile mean |d| (%dx%d grid):" % (GX, GY))
for gy in range(GY):
    row = []
    for gx in range(GX):
        cell = dmax[gy * h // GY:(gy + 1) * h // GY, gx * w // GX:(gx + 1) * w // GX]
        row.append("%5.2f" % cell.mean())
    print("   " + " ".join(row))

# ---- contact sheet: before | after | 8x difference -----------------------
tw = b.width // 2
th = b.height // 2
diff = ImageChops.difference(a, b).point(lambda v: min(255, v * 8))
sheet = Image.new("RGB", (tw * 3, th + 26), (18, 18, 20))
for i, (img, label) in enumerate(((b, "BEFORE  MAT_MAPS=off"),
                                  (a, "AFTER  authored _n/_r"),
                                  (diff, "|diff| x8"))):
    sheet.paste(img.resize((tw, th), Image.LANCZOS), (i * tw, 26))
    ImageDraw.Draw(sheet).text((i * tw + 8, 8), label, fill=(235, 235, 235))
sheet.save(SHEET)
print("sheet       : %s" % SHEET)
