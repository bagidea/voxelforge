#!/usr/bin/env python3
"""Kevin — render the corrected REF vs OURS figure for the art-gap report.

Series: REF (single middle panel, primary), OURS interior (beauty, g4),
and OURS outdoor (like-for-like). 2x3 grid: hue histogram, warm/cool/neutral
split, saturation, luminance, edge density, dominant colors.
"""
import os
os.environ["LOKY_MAX_CPU_COUNT"] = "4"
import sys
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure, hue_sat_val, luma

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SERIES = {
    "REF_mid":       ("REF (single panel)", os.path.join(ROOT, "_kevin_ref_panel_mid.png"), "#1f77b4"),
    "beauty":        ("OURS beauty",          os.path.join(ROOT, r"_fl_beauty_20260816\beauty-wide-nohud2.png"), "#d62728"),
    "g4":            ("OURS g4",              os.path.join(ROOT, r"_fl_g2g4_20260816\g4-only-nohud2.png"), "#2ca02c"),
    "outdoor_noon":  ("OURS outdoor",         os.path.join(ROOT, r"_fl_lookv4\outdoor-noon-after-nohud2.png"), "#9467bd"),
    # blue-sky proof bar — only added to the warm/cool panel (not the other 5) so the
    # figure refutes "no cool channel exists" at a glance instead of leaving 0% bars.
    "skyh25":        ("OURS blue sky",        os.path.join(ROOT, r"_flamingo_g6\skyh25-nohud2.png"), "#17becf"),
}


def warm_cool_neutral(a):
    h, s, _ = hue_sat_val(a)
    sm = s >= 0.08
    warm = ((h < 70) | (h >= 340)) & sm
    cool = (h >= 170) & (h < 270) & sm
    tot = int(sm.sum())
    return (100.0 * warm.sum() / tot, 100.0 * cool.sum() / tot,
            100.0 * (sm & ~warm & ~cool).sum() / tot)


rows = {}
for key, (label, path, color) in SERIES.items():
    r = measure(path, key)
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    r["label"], r["color"] = label, color
    r["warm_pct"], r["cool_pct"], r["neutral_pct"] = warm_cool_neutral(a)
    rows[key] = r

order = ["REF_mid", "beauty", "g4", "outdoor_noon"]
order_wc = order + ["skyh25"]  # warm/cool panel gets the blue-sky proof bar
fig, ax = plt.subplots(2, 3, figsize=(18, 9.5))
fig.suptitle("Art gap vs reference — corrected 2026-08-17 (REF = single panel, a render, not a photo)",
             fontsize=13, fontweight="bold")

# 1. hue histogram
axh = ax[0][0]
for k in order:
    r = rows[k]
    axh.plot(np.arange(360), r["h360"] * 100, color=r["color"], label=r["label"], lw=1.6)
axh.set_title("Hue histogram (saturated px, 10° bins = 0.5% of mass)")
axh.set_xlabel("Hue (deg)  0=red 60=yellow 120=green 180=cyan 240=blue 300=magenta")
axh.set_ylabel("% of saturated pixels")
axh.set_xlim(0, 360)
axh.legend(fontsize=8)

# 2. warm / cool / neutral stacked bars
axw = ax[0][1]
x = np.arange(len(order_wc))
bottom = np.zeros(len(order_wc))
for comp, c, lbl in (("warm_pct", "#e24a33", "warm"), ("cool_pct", "#348abd", "cool"),
                     ("neutral_pct", "#999999", "neutral")):
    vals = np.array([rows[k][comp] for k in order_wc])
    axw.bar(x, vals, bottom=bottom, color=c, label=lbl, width=0.62)
    for i, v in enumerate(vals):
        if v > 2:
            axw.text(i, bottom[i] + v / 2, "%.0f%%" % v, ha="center", va="center",
                     fontsize=8, color="white" if comp != "neutral_pct" else "black")
    bottom += vals
axw.set_xticks(x)
axw.set_xticklabels([rows[k]["label"] for k in order_wc], fontsize=8)
axw.set_ylim(0, 105)
axw.set_ylabel("% of saturated pixels")
axw.set_title("Warm / cool / neutral — 3 shipped plates 0% cool; our own blue-sky render hits 19%")
axw.legend(fontsize=8, loc="upper right")

# 3. saturation histogram
axs = ax[0][2]
for k in order:
    r = rows[k]
    axs.hist(r["sat"].ravel(), bins=64, range=(0, 1), density=True, alpha=0.45,
             color=r["color"], label=r["label"])
axs.set_title("Saturation histogram (all px)")
axs.set_xlabel("Saturation (0..1)")
axs.set_ylabel("density")
axs.legend(fontsize=8)

# 4. luminance histogram with zones
axl = ax[1][0]
for k in order:
    r = rows[k]
    axl.hist(r["L"].ravel(), bins=128, range=(0, 255), density=True, alpha=0.45,
             color=r["color"], label=r["label"])
axl.axvspan(0, 85, color="black", alpha=0.10)
axl.axvspan(85, 170, color="gray", alpha=0.08)
axl.axvspan(170, 255, color="white", alpha=0.10)
axl.text(42, axl.get_ylim()[1] * 0.95, "shadow", ha="center", fontsize=8)
axl.text(127, axl.get_ylim()[1] * 0.95, "midtone", ha="center", fontsize=8)
axl.text(212, axl.get_ylim()[1] * 0.95, "highlight", ha="center", fontsize=8)
axl.set_title("Luminance histogram (Rec.601, 0..255)")
axl.set_xlabel("Luminance")
axl.set_ylabel("density")
axl.legend(fontsize=8)

# 5. edge density bar (mean + strong)
axe = ax[1][1]
x = np.arange(len(order))
edge_mean = [rows[k]["edge_mean"] for k in order]
strong = [rows[k]["strong"] * 100 for k in order]
colors = [rows[k]["color"] for k in order]
bars = axe.bar(x, edge_mean, color=colors, width=0.62, alpha=0.85)
for i, (em, st) in enumerate(zip(edge_mean, strong)):
    axe.text(i, em + 1.2, "%.1f\n(%.0f%% strong)" % (em, st), ha="center", va="bottom", fontsize=8)
axe.axhline(30, color="gray", ls="--", lw=1)
axe.text(len(order) - 0.5, 31, "Pile-B target 30", fontsize=8, color="gray", ha="right")
axe.set_xticks(x)
axe.set_xticklabels([rows[k]["label"] for k in order], fontsize=8)
axe.set_ylim(0, max(edge_mean) * 1.28)
axe.set_ylabel("edge mean (Sobel, /px)")
axe.set_title("Edge density — REF 60.5 is a render value; our outdoor already hits 36.8")

# 6. dominant color swatches (k=8)
axc = ax[1][2]
axc.axis("off")
axc.set_title("Dominant colors (k-means k=8)", fontsize=11)
for i, k in enumerate(order):
    r = rows[k]
    y = len(order) - 1 - i
    axc.text(-0.02, y, r["label"], fontsize=8, va="center", ha="right",
             transform=axc.get_yaxis_transform())
    for j, (hx, m) in enumerate(zip(r["centers"], r["mass"])):
        xx = j * 0.125
        axc.add_patch(plt.Rectangle((xx, y - 0.28), 0.12, 0.56, facecolor=hx,
                                    edgecolor="#333", linewidth=0.5))
        axc.text(xx + 0.06, y - 0.36, "%.0f%%" % (m * 100), fontsize=6, ha="center", va="top")
axc.set_xlim(0, 1)
axc.set_ylim(-0.5, len(order) - 0.4)

fig.tight_layout(rect=[0, 0, 1, 0.96])
out = os.path.join(ROOT, "docs", "art-gap-vs-reference-2026-08-17.png")
fig.savefig(out, dpi=130)
print("WROTE", out)
