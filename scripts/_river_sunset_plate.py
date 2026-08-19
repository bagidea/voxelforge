#!/usr/bin/env python3
"""_river_sunset_plate.py — the second-scene overfit plate, one glance.

Row 1: CEO reference | beach_dusk (_matmaps_after, the overfit frame) | river_sunset
Row 2: the grader's own masks for those three (sky blue / far pink / near green)
Row 3: the axes that FLIP between beach_dusk and river_sunset — that flip IS the overfit.

Run:  python scripts/_river_sunset_plate.py
Out:  docs/assets/artgap/river-sunset-vs-ceo-ref-2026-08-19.png
"""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.gridspec import GridSpec
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
OUT = ROOT / "docs" / "assets" / "artgap" / "river-sunset-vs-ceo-ref-2026-08-19.png"

REF = ROOT / "docs" / "refs" / "ceo_ref_sunset_valley.jpg"
BEACH = ROOT / "_matmaps_after.png"
RIVER = ROOT / "_river_sunset_after.png"
MASKS = ROOT / "docs" / "assets" / "artgap" / "river_sunset" / "masks"

GATE = 0.60

# (label, beach ratio, river ratio, kind)
# kind: overfit  = pass at beach, GAP at river      (red)
#       undersold = GAP at beach, pass at river     (green)
#       new       = SKIP at beach, measurable at river (violet)
#       advisory  = ok at beach, "over" (advisory) at river (amber)
FLIPS = [
    ("sky brighter than ground", 0.09, 0.65, "undersold"),
    ("crushed blacks (less better)", 0.20, 1.94, "undersold"),
    ("hue diversity", 0.55, 0.61, "undersold"),
    ("distant silhouette", None, 2.38, "new"),
    ("sky presence", 0.83, 1.75, "advisory"),
    ("flat / featureless area (less better)", 0.63, 0.44, "overfit"),
    ("local contrast r3", 0.62, 0.45, "overfit"),
    ("dynamic range", 0.69, 0.55, "overfit"),
    ("tonal spread", 0.66, 0.53, "overfit"),
]
KIND_COLOR = {"overfit": "#e2483f", "undersold": "#3fa96b",
              "new": "#8f6fe0", "advisory": "#e08b2f"}


def fit(path, h):
    im = Image.open(path).convert("RGB")
    w = round(im.width * h / im.height)
    return np.asarray(im.resize((w, h), Image.LANCZOS))


def main():
    data = json.loads((ROOT / "docs" / "assets" / "artgap" / "river_sunset" / "artgap.json").read_text())
    beach = next(f for f in data["frames"] if f["file"] == "_matmaps_after.png")
    river = next(f for f in data["frames"] if f["file"] == "_river_sunset_after.png")

    fig = plt.figure(figsize=(22, 20), facecolor="#12131a")
    gs = GridSpec(2, 3, figure=fig, height_ratios=[2.6, 1.0],
                  hspace=0.28, wspace=0.03, left=0.02, right=0.98, top=0.915, bottom=0.42)
    gs_bar = GridSpec(1, 1, figure=fig, left=0.31, right=0.985, top=0.375, bottom=0.05)

    fig.suptitle("Voxelforge — the overfit test: a second outdoor scene  ·  2026-08-19",
                 color="#f2f3f7", fontsize=26, fontweight="bold", y=0.985)
    fig.text(0.5, 0.958,
             "left: docs/refs/ceo_ref_sunset_valley.jpg   ·   middle: _matmaps_after.png "
             "(beach_dusk — the ONE frame the office grades)   ·   right: _river_sunset_after.png "
             "(second scene, 14350 blocks incl. 2011 water, MAP_APPLY skipped=0)   ·   "
             "controls exit 0 (rule 7)",
             color="#9aa0b4", fontsize=11.5, ha="center")

    H = 1000
    for col, (path, title, m) in enumerate([
            (REF, "CEO REFERENCE", None),
            (BEACH, "BEACH_DUSK — _matmaps_after.png", beach),
            (RIVER, "RIVER_SUNSET — _river_sunset_after.png", river)]):
        ax = fig.add_subplot(gs[0, col])
        ax.imshow(fit(path, H))
        ax.set_title(title, color="#f2f3f7", fontsize=15.5, fontweight="bold", pad=9)
        ax.axis("off")
        if m is not None:
            cap = (f"sky/ground {m['sky_ground_ratio']:.2f}x   ·   sky black {m['sky_void_pct']:.1f}%\n"
                   f"dyn range {m['range_p5_p95']:.0f}   ·   detail/area {m['edge_density']:.1f}\n"
                   f"silhouette: {'SKIP (black sky)' if m['far_edge_contrast'] is None else f'{m['far_edge_contrast']:.1f} vs ref 22.3'}")
            fig.text(0.04 + col * 0.32, 0.545, cap, ha="left", va="top",
                     color="#c7ccdb", fontsize=12.5, family="monospace", linespacing=1.6)

    for col, path in enumerate([REF, BEACH, RIVER]):
        ax = fig.add_subplot(gs[1, col])
        mk = MASKS / (path.stem + "_mask.png")
        if mk.exists():
            ax.imshow(fit(mk, 420))
        ax.axis("off")
    fig.text(0.5, 0.435,
             "grader's own mask — sky (blue) · far field (pink) · near field (green) · horizon (yellow)",
             color="#9aa0b4", fontsize=12.5, ha="center")

    # ---- the flip chart ---------------------------------------------------
    ax = fig.add_subplot(gs_bar[0, 0])
    ax.set_facecolor("#12131a")
    labels = [f[0] for f in FLIPS]
    y = np.arange(len(labels))[::-1]
    barh = 0.32
    for i, (label, b, r, kind) in enumerate(FLIPS):
        yy = y[i]
        c = KIND_COLOR[kind]
        # beach bar
        if b is None:
            ax.barh(yy + barh / 2, 1.0, height=barh, color="#3a3f4d",
                    edgecolor="#555a6b", hatch="//")
            ax.text(0.04, yy + barh / 2, "SKIP (refused)", va="center",
                    color="#9aa0b4", fontsize=10)
        else:
            ax.barh(yy + barh / 2, min(b, 1.95), height=barh, color=c, alpha=0.55)
            ax.text(b + 0.02, yy + barh / 2, f"{b:.2f}x", va="center",
                    color="#c7ccdb", fontsize=10.5)
        # river bar
        ax.barh(yy - barh / 2, min(r, 1.95), height=barh, color=c)
        ax.text(r + 0.02, yy - barh / 2, f"{r:.2f}x", va="center",
                color="#f2f3f7", fontsize=10.5)

    ax.axvline(GATE, color="#f2f3f7", ls="--", lw=1.5, alpha=0.8)
    ax.text(GATE + 0.01, len(labels) - 0.35, "gate = 60%", color="#f2f3f7", fontsize=11)
    ax.axvline(1.0, color="#6d7488", ls=":", lw=1.3)
    ax.text(1.01, len(labels) - 0.35, "REF", color="#9aa0b4", fontsize=11)
    ax.set_yticks(y)
    ax.set_yticklabels(labels, color="#f2f3f7", fontsize=12.5)
    ax.set_xlim(0, 2.45)
    ax.set_ylim(-1.1, len(labels) - 0.15)
    ax.set_xticks([0, 0.6, 1.0, 1.67, 2.0])
    ax.set_xticklabels(["0", "60%", "REF", "167%", "2x"], color="#9aa0b4")
    ax.tick_params(axis="y", length=0)
    for s in ax.spines.values():
        s.set_visible(False)
    ax.set_title("axes that FLIP from beach_dusk to river_sunset — the flip is the overfit",
                 color="#f2f3f7", fontsize=16.5, fontweight="bold", pad=14, loc="left")

    handles = [plt.Line2D([], [], marker="s", ls="", ms=13, color=c, label=k)
               for k, c in KIND_COLOR.items()]
    ax.legend(handles=handles, loc="upper right", frameon=False,
              labelcolor="#c7ccdb", fontsize=11, ncol=4,
              bbox_to_anchor=(1.0, -0.01))
    fig.text(0.02, 0.05,
             "faded bar = beach_dusk   ·   solid bar = river_sunset   ·   dashed = 60% gate   ·   "
             "167% = the 'over' advisory line (as far above REF as a GAP is below it)",
             color="#7f8496", fontsize=11.5, ha="left", style="italic")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(OUT, dpi=68, facecolor=fig.get_facecolor())
    print(f"wrote {OUT}  ({OUT.stat().st_size/1e6:.2f} MB)")


if __name__ == "__main__":
    main()
