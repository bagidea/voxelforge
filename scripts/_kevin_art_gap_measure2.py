#!/usr/bin/env python3
"""Kevin — art-gap vs reference, measurement lane (NO build / NO cargo).

Extended pass (2026-08-17 correction): adds single-panel REF (middle, sharpest)
and the outdoor like-for-like plates, and records warm/cool/neutral % split.
Same pinned metric definitions as _kevin_art_gap_measure.py.
"""
import os
os.environ["LOKY_MAX_CPU_COUNT"] = "4"
import sys
import numpy as np
import json
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure, hue_sat_val

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REF_UPLOAD = r"E:\Projects\bagidea-ai-agents-office\workspace\uploads\1786952280558_773223404_122312179466226047_9178907560287775136_n.jpg"
REF_MID_PATH = os.path.join(ROOT, "_kevin_ref_panel_mid.png")


def ensure_ref_mid_crop():
    """Crop the single middle panel out of the 3-panel REF stack, if not done yet.

    The reference is 768x1376 with two near-white separators at rows 459-460 and
    916-917 (detected as rows with std < 8). The middle panel (rows 461-915) is the
    sharpest of the three and is the single-frame primary for the report.
    """
    if os.path.exists(REF_MID_PATH):
        return
    a = np.asarray(Image.open(REF_UPLOAD).convert("RGB"), dtype=np.uint8)
    Image.fromarray(a[461:916]).save(REF_MID_PATH)
    print("cropped", REF_MID_PATH, "rows 461:916")


ensure_ref_mid_crop()

paths = {
    "REF_full": REF_UPLOAD,
    "REF_mid":  REF_MID_PATH,
    "beauty":   os.path.join(ROOT, r"_fl_beauty_20260816\beauty-wide-nohud2.png"),
    "g4":       os.path.join(ROOT, r"_fl_g2g4_20260816\g4-only-nohud2.png"),
    "outdoor_noon":     os.path.join(ROOT, r"_fl_lookv4\outdoor-noon-after-nohud2.png"),
    "crop_outdoor_sky": os.path.join(ROOT, r"_fl_lookv4\crop-outdoor-noon-far-terrain-sky.png"),
}

rows = {}
for name, p in paths.items():
    r = measure(p, name)
    a = np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)
    h, s, _ = hue_sat_val(a)
    sat_mask = s >= 0.08
    warm = ((h < 70) | (h >= 340)) & sat_mask
    cool = (h >= 170) & (h < 270) & sat_mask
    tot = int(sat_mask.sum())
    r["warm_pct"] = 100.0 * warm.sum() / tot
    r["cool_pct"] = 100.0 * cool.sum() / tot
    r["neutral_pct"] = 100.0 * (sat_mask & ~warm & ~cool).sum() / tot
    rows[name] = r

keys = ["hue90", "occupied", "sat_mean", "sat_std", "shadow", "midtone", "highlight",
        "lum_mean", "edge_mean", "strong", "warmcool", "warm_pct", "cool_pct", "neutral_pct"]
print("%-16s %s" % ("metric", "  ".join("%14s" % n for n in rows)))
for k in keys:
    vals = []
    for n in rows:
        v = rows[n][k]
        vals.append("%14s" % (("%.3f" % v) if np.isfinite(v) else "inf"))
    print("%-16s %s" % (k, "  ".join(vals)))

out = {n: {k: rows[n][k] for k in keys} for n in rows}
with open(os.path.join(ROOT, "_kevin_art_gap.json"), "w") as f:
    json.dump(out, f, indent=2)
print("\nWROTE _kevin_art_gap.json")

for n in ("REF_mid", "outdoor_noon", "crop_outdoor_sky"):
    print("\n%s centers:" % n,
          " ".join("%s %.0f%%" % (hx, m * 100) for hx, m in zip(rows[n]["centers"], rows[n]["mass"])))
