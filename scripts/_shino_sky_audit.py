"""Shino — independent audit of Kevin's sky* cool% claim (2026-08-17).

Written from scratch (matplotlib.colors.rgb_to_hsv instead of a hand-rolled
hue/sat), same pinned split: saturated px (s>=0.08), cool = hue 170..270.
If Kevin's table is right, these numbers land on his to ~0.1.
"""
import os
import numpy as np
from PIL import Image
from matplotlib.colors import rgb_to_hsv

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

PLATES = [
    "_fl_lookv4/outdoor-noon-after-nohud2.png",   # the shipped outdoor plate (claim: cool 0%)
    "_flamingo_g6/skyh15-nohud2.png",
    "_flamingo_g6/skyh25-nohud2.png",
    "_flamingo_g7/sky2-nohud2.png",
    "_flamingo_g7/sky3-nohud2.png",
    "_flamingo_g7/sky4-nohud2.png",
    "_flamingo_g7/ev097sky3-nohud2.png",
    "_flamingo_g7/ev100sky3-nohud2.png",
    "_kevin_ref_panel_mid.png",                    # REF single panel (claim: 17%)
]

print("%-46s %7s %7s  %-22s %s" % ("plate", "cool%", "warm%", "top10% band RGB", "blue?"))
for rel in PLATES:
    p = os.path.normpath(os.path.join(ROOT, rel))
    if not os.path.exists(p):
        print("%-46s  MISSING" % rel)
        continue
    a = np.asarray(Image.open(p).convert("RGB"), dtype=np.float64) / 255.0
    hsv = rgb_to_hsv(a)
    hue = hsv[..., 0] * 360.0
    sat = hsv[..., 1]
    m = sat >= 0.08
    cool = ((hue >= 170) & (hue < 270)) & m
    warm = ((hue < 70) | (hue >= 340)) & m
    tot = int(m.sum())
    band = a[0:int(a.shape[0] * 0.10)].reshape(-1, 3).mean(axis=0) * 255.0
    print("%-46s %7.1f %7.1f  %-22s %s" % (
        rel,
        100.0 * cool.sum() / tot,
        100.0 * warm.sum() / tot,
        "(%d, %d, %d)" % tuple(int(x) for x in band),
        "B>R yes" if band[2] > band[0] else "NO (warm)",
    ))
