#!/usr/bin/env python3
"""_pixel_artgap_plate.py — one plate that shows the art gap in a single glance.

Row 1: CEO reference vs our frame, matched height (NOT matched framing - see caveat)
Row 2: the sky mask both graders actually measured, so the region is falsifiable
Row 3: every axis as a share of the reference, sorted worst-first

Run:  python scripts/_pixel_artgap_plate.py
Out:  docs/assets/artgap/art-gap-vs-ceo-ref-2026-08-18.png
"""

from __future__ import annotations

import importlib.util
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
OUT = ROOT / "docs" / "assets" / "artgap" / "art-gap-vs-ceo-ref-2026-08-18.png"

REF = ROOT / "docs" / "refs" / "ceo_ref_sunset_valley.jpg"
OURS = ROOT / "_matmaps_after.png"
MASKS = ROOT / "docs" / "assets" / "artgap" / "masks"

spec = importlib.util.spec_from_file_location("artgap", HERE / "_pixel_artgap_grade.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)

# the eight gaps, ranked by beauty-per-unit-of-effort (see the report for the why)
RANKED = [
    ("sky_ground_ratio", "G1  sky brighter than ground"),
    ("emissive_blobs", "G2  emissive light points"),
    ("cool_chroma_pct", "G3  cool / water chroma"),
    ("sky_L_range", "G4  sky tonal gradient"),
    ("sky_void_pct", "G5  sky sitting at black"),
    ("hue_bins", "G6  palette breadth (hue bins)"),
    ("edge_density", "G7  detail per unit area"),
    ("crush_pct", "G8  crushed blacks"),
    ("sky_hue_span_deg", "G9  sky hue range"),
    ("far_edge_contrast", "G10 distant silhouette"),
    ("micro_r3", "G11 local contrast"),
    ("range_p5_p95", "G12 dynamic range p5-p95"),
]
DIRS = {k: d for k, d, _, _ in G.AXES}
OWNER = {k: o for k, d, _, o in G.AXES}
# the bars are coloured by SEVERITY, not by owner - the owner is printed inside
# each label instead. An owner-keyed legend here would be a caption that does not
# match its own pixels.
SEVERITY = [("#e2483f", "under 30% of reference"),
            ("#e08b2f", "30-60% - below the gate"),
            ("#3fa96b", "at or above the 60% gate"),
            ("#555a6b", "unmeasurable (grader refused)")]


def fit(path, h):
    im = Image.open(path).convert("RGB")
    w = round(im.width * h / im.height)
    return np.asarray(im.resize((w, h), Image.LANCZOS))


def main():
    ref = G.measure(str(REF))
    ours = G.measure(str(OURS))

    fig = plt.figure(figsize=(20, 21), facecolor="#12131a")
    # two grids: the image rows want the full width, the bar chart wants a wide
    # left gutter for its axis labels
    gs = GridSpec(2, 2, figure=fig, height_ratios=[3.0, 1.3],
                  hspace=0.30, wspace=0.02, left=0.02, right=0.98, top=0.925, bottom=0.435)
    gs_bar = GridSpec(1, 1, figure=fig, left=0.255, right=0.985, top=0.385, bottom=0.045)

    fig.suptitle("Voxelforge — art gap vs the CEO reference, measured  ·  2026-08-18",
                 color="#f2f3f7", fontsize=27, fontweight="bold", y=0.985)
    fig.text(0.5, 0.962,
             "left: docs/refs/ceo_ref_sunset_valley.jpg   ·   right: _matmaps_after.png "
             "(beach_dusk, binary sha256 e7db71ca…)   ·   grader: scripts/_pixel_artgap_grade.py "
             "— 21/21 self-grade on the reference; C0 identity + 6 lesions pass",
             color="#9aa0b4", fontsize=12.5, ha="center")

    H = 1100
    for col, (path, title, m) in enumerate([
            (REF, "CEO REFERENCE", ref), (OURS, "OURS — _matmaps_after.png", ours)]):
        ax = fig.add_subplot(gs[0, col])
        ax.imshow(fit(path, H))
        ax.set_title(title, color="#f2f3f7", fontsize=19, fontweight="bold", pad=10)
        ax.axis("off")
        # figure coords, not axes coords: imshow shrinks each axes box to its own
        # aspect, so an axes-relative caption lands at a different height per column
        caption = (f"sky is {fmt(m['sky_ground_ratio'])}x the brightness of the ground\n"
                   f"{m['emissive_blobs']} emissive points   ·   {m['hue_bins']}/36 hue bins\n"
                   f"dyn range {fmt(m['range_p5_p95'])}   ·   detail/area {fmt(m['edge_density'])}")
        fig.text(0.26 + col * 0.48, 0.633, caption, ha="center", va="top",
                 color="#c7ccdb", fontsize=14, family="monospace", linespacing=1.7)

    for col, (path, m) in enumerate([(REF, ref), (OURS, ours)]):
        ax = fig.add_subplot(gs[1, col])
        mk = MASKS / (Path(path).stem + "_mask.png")
        if mk.exists():
            ax.imshow(fit(mk, 520))
        ax.axis("off")
        ax.set_title("grader's own mask — sky (blue) · far field (pink) · near field (green)",
                     color="#9aa0b4", fontsize=13, pad=8)

    # ---- ranked gap bars -------------------------------------------------
    ax = fig.add_subplot(gs_bar[0, 0])
    ax.set_facecolor("#12131a")
    labels, vals, colors, notes = [], [], [], []
    for key, label in RANKED:
        r = G.ratio(ours[key], ref[key], DIRS[key])
        labels.append(f"{label}   [{OWNER[key]}]")
        if r is None:
            vals.append(0.0)
            colors.append("#555a6b")
            notes.append("UNMEASURABLE — sky unlit, edge contrast would be degenerate")
        else:
            vals.append(min(r, 1.05))
            colors.append("#e2483f" if r < 0.30 else "#e08b2f" if r < G.GATE else "#3fa96b")
            notes.append(f"{fmt(ours[key])}  vs ref {fmt(ref[key])}   =  {r*100:.0f}% of reference")

    y = np.arange(len(labels))[::-1]
    ax.barh(y, vals, color=colors, height=0.66, edgecolor="none")
    ax.axvline(G.GATE, color="#f2f3f7", ls="--", lw=1.6, alpha=0.75)
    ax.text(G.GATE + 0.012, len(labels) - 0.4, "gate = 60% of reference",
            color="#f2f3f7", fontsize=12, alpha=0.85)
    ax.axvline(1.0, color="#6d7488", ls=":", lw=1.4)
    ax.text(1.012, len(labels) - 0.4, "REF", color="#9aa0b4", fontsize=12)
    for yy, n in zip(y, notes):
        ax.text(1.085, yy, n, va="center", color="#c7ccdb", fontsize=12.5, family="monospace")
    ax.set_yticks(y)
    ax.set_yticklabels(labels, color="#f2f3f7", fontsize=13.5)
    ax.set_xlim(0, 1.98)
    # one empty row below the last bar so the severity key never lands on a note
    ax.set_ylim(-1.55, len(labels) - 0.35)
    ax.set_xticks([0, 0.25, 0.5, 0.6, 0.75, 1.0])
    ax.set_xticklabels(["0", "25%", "50%", "60%", "75%", "REF"], color="#9aa0b4")
    ax.tick_params(axis="y", length=0)
    for s in ax.spines.values():
        s.set_visible(False)
    ax.set_title("every axis as a share of the CEO reference — worst first",
                 color="#f2f3f7", fontsize=18, fontweight="bold", pad=16, loc="left")

    handles = [plt.Line2D([], [], marker="s", ls="", ms=13, color=c, label=k)
               for c, k in SEVERITY]
    ax.legend(handles=handles, loc="lower right", frameon=False,
              labelcolor="#c7ccdb", fontsize=12, ncol=4,
              bbox_to_anchor=(1.0, -0.005),
              title="bar colour = severity   ·   the owning lane is inside each label",
              title_fontsize=12)
    ax.get_legend().get_title().set_color("#9aa0b4")

    fig.text(0.5, 0.010,
             "FRAMING CAVEAT: the reference is 768x1376 portrait, ours is 1280x720. Both are "
             "resampled to a common 1.0 Mpx AREA, so per-area metrics compare honestly; "
             "composition and shot length do NOT. Nothing here is scored on framing.",
             color="#7f8496", fontsize=12, ha="center", style="italic")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(OUT, dpi=68, facecolor=fig.get_facecolor())
    print(f"wrote {OUT}  ({OUT.stat().st_size/1e6:.2f} MB)")


def fmt(v):
    return "-" if v is None else (f"{v:.1f}" if isinstance(v, float) else str(v))


if __name__ == "__main__":
    main()
