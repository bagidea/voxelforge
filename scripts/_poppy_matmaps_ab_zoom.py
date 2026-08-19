#!/usr/bin/env python3
"""Poppy - the zoom sheet. The full frame hides the lever; the crops don't.

Five 160x120 patches, each cut from BOTH plates at the SAME rect, stacked
before-over-after and blown up 3x nearest-neighbour so nothing is invented by
the resampler. Under each pair: that patch's own mean |dRGB| and its local
contrast (std of the luma high-pass), because "the maps add relief" is a claim
about local contrast and should be printed as one.

USAGE  python scripts/_poppy_matmaps_ab_zoom.py
"""
import os

from PIL import Image, ImageDraw
import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
b = Image.open(os.path.join(ROOT, "_matmaps_before.png")).convert("RGB")
a = Image.open(os.path.join(ROOT, "_matmaps_after.png")).convert("RGB")

# (label, x, y, w, h) - picked off the frame, not off the difference.
CROPS = [
    ("brick + mortar", 620, 190, 160, 120),
    ("plaster + glass", 400, 350, 160, 120),
    ("log end-grain", 260, 560, 160, 120),
    ("sand", 880, 520, 160, 120),
    ("leaves", 1090, 330, 160, 120),
]
Z = 3
CW, CH = 160 * Z, 120 * Z
PAD, HDR, FTR = 10, 30, 46

sheet = Image.new("RGB", (len(CROPS) * (CW + PAD) + PAD, HDR + 2 * (CH + 20) + FTR), (16, 16, 18))
dr = ImageDraw.Draw(sheet)


def local_contrast(img):
    """std of (luma - 3x3 box mean). Relief raises it; a flat repaint does not."""
    g = np.asarray(img.convert("L"), dtype=np.float32)
    k = np.ones((3, 3), np.float32) / 9.0
    p = np.pad(g, 1, mode="edge")
    box = sum(p[i:i + g.shape[0], j:j + g.shape[1]] * k[i, j]
              for i in range(3) for j in range(3))
    return float((g - box).std())


print("%-16s %8s %10s %10s %8s" % ("patch", "mean|d|", "contr_bef", "contr_aft", "delta%"))
for i, (label, x, y, w, h) in enumerate(CROPS):
    cb = b.crop((x, y, x + w, y + h))
    ca = a.crop((x, y, x + w, y + h))
    ox = PAD + i * (CW + PAD)
    dr.text((ox, 8), label, fill=(240, 240, 240))
    sheet.paste(cb.resize((CW, CH), Image.NEAREST), (ox, HDR))
    dr.text((ox + 4, HDR + 2), "BEFORE", fill=(255, 210, 120))
    sheet.paste(ca.resize((CW, CH), Image.NEAREST), (ox, HDR + CH + 20))
    dr.text((ox + 4, HDR + CH + 22), "AFTER", fill=(140, 255, 180))

    d = np.abs(np.asarray(ca, np.int16) - np.asarray(cb, np.int16)).mean()
    lb, la = local_contrast(cb), local_contrast(ca)
    pct = (la - lb) / lb * 100.0 if lb else 0.0
    print("%-16s %8.2f %10.2f %10.2f %+7.1f%%" % (label, d, lb, la, pct))
    dr.text((ox, HDR + 2 * CH + 26),
            "mean|d| %.2f   contrast %.1f -> %.1f (%+.0f%%)" % (d, lb, la, pct),
            fill=(200, 200, 205))

out = os.path.join(ROOT, "_matmaps_zoom_sheet.png")
sheet.save(out)
print("sheet: %s" % out)
