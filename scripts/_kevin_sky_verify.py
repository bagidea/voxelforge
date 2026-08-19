#!/usr/bin/env python3
"""Kevin — verify cool% of the _flamingo_g6/_g7 sky* plates (2026-08-17 reviewer note).

One-off check: do these plates already hit REF-level cool share (~13-19%)? Uses the
same pinned warm/cool split as _kevin_art_gap_measure2.py (saturated px, hue bins).
"""
import os
import sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import hue_sat_val

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PLATES = [
    "_flamingo_g6/skyh15-nohud2.png",
    "_flamingo_g6/skyh25-nohud2.png",
    "_flamingo_g6/skybh2-nohud2.png",
    "_flamingo_g6/skybib-nohud2.png",
    "_flamingo_g7/sky2-nohud2.png",
    "_flamingo_g7/sky3-nohud2.png",
    "_flamingo_g7/sky4-nohud2.png",
    "_flamingo_g7/ev097sky3-nohud2.png",
    "_flamingo_g7/ev100sky3-nohud2.png",
]

hdr = "%-34s %8s %8s %8s  %s" % ("plate", "cool%", "warm%", "neutral%", "sky-band(top10%) RGB")
print(hdr)
for rel in PLATES:
    p = os.path.join(ROOT, rel)
    a = np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)
    h, s, _ = hue_sat_val(a)
    mask = s >= 0.08
    warm = ((h < 70) | (h >= 340)) & mask
    cool = (h >= 170) & (h < 270) & mask
    tot = int(mask.sum())
    cp = 100.0 * cool.sum() / tot
    wp = 100.0 * warm.sum() / tot
    npct = 100.0 * (mask & ~warm & ~cool).sum() / tot
    band = a[0:int(a.shape[0] * 0.10)]
    rgb = tuple(int(x) for x in band.reshape(-1, 3).mean(axis=0))
    print("%-34s %8.1f %8.1f %8.1f  %s" % (rel, cp, wp, npct, rgb))
