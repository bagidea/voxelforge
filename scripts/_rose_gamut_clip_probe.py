#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Rose — gamut-clip probe for the _pixel_shotset_N6 shotset.

Investigates whether the `saturation (mid)` P0 axis (scripts/grade_axes.py) is
trustworthy, given Flamingo's observation (commit 40cf97e) that saturation(mid)
~= 100.0 and blue B(mid) ~= 0.0 land on most plates identically, before AND after.

Hypothesis (read from grade_axes.measure): saturation is computed per pixel as
HSV-V sat = (max-min)/max. A pixel whose minimum channel is clipped to 0 scores
sat = (max-0)/max = 1.0 = 100%, so a midtone band whose B has been crushed to 0
by gamut clipping reads as "perfectly saturated" -- the gate scores the *damage*.

This probe counts REAL clipped pixels (==0 and ==255) per channel at NATIVE
resolution, then drills into the midtone band (luminance p35..p75, identical to
grade_axes) so the clip picture is on the exact population the gate grades. It
prints the current HSV sat mean (the suspect) next to an "honest" sat that
excludes railed pixels, and the golden ref as a positive control (a known-good
frame must NOT trip an honest gate).

Pure Python + PIL/numpy. No cargo, no build, no exe run -- reads PNGs only.
"""
import os
import sys
import glob
import numpy as np
from PIL import Image


def _label(path):
    p = path.replace("\\", "/")
    if "golden-beauty-shot-ref" in p:
        return "REF " + os.path.basename(p)
    if "/before/" in p:
        return "BEF " + os.path.basename(p)
    if "/after/" in p:
        return "AFT " + os.path.basename(p)
    return "--- " + os.path.basename(p)


def analyze(path):
    im = Image.open(path).convert("RGB")  # native resolution -- count real pixels
    a = np.asarray(im).astype(np.float32)
    H, W, _ = a.shape
    n = H * W
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126 * R + 0.7152 * G + 0.0722 * B

    # per-channel rails (whole frame, native)
    rails = {}
    for name, ch in [("R", R), ("G", G), ("B", B)]:
        rails[name] = (float((ch == 0).sum()) / n * 100.0,
                       float((ch == 255).sum()) / n * 100.0)
    any0 = float(((R == 0) | (G == 0) | (B == 0)).sum()) / n * 100.0
    any255 = float(((R == 255) | (G == 255) | (B == 255)).sum()) / n * 100.0

    # midtone band -- same definition as grade_axes.measure (L p35..p75)
    tlo, thi = np.percentile(L, 35), np.percentile(L, 75)
    mm = (L >= tlo) & (L <= thi)
    nmid = int(mm.sum())
    Rm, Gm, Bm = R[mm], G[mm], B[mm]
    mx = np.maximum(np.maximum(Rm, Gm), Bm)
    mn = np.minimum(np.minimum(Rm, Gm), Bm)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    cur_sat = float(sat.mean() * 100.0)

    mid_B0 = float((Bm == 0).sum()) / nmid * 100.0
    mid_B_le2 = float((Bm <= 2).sum()) / nmid * 100.0
    mid_any0 = float(((Rm == 0) | (Gm == 0) | (Bm == 0)).sum()) / nmid * 100.0
    mid_meanB = float(Bm.mean())

    # honest sat: exclude railed pixels (any channel at 0 or 255) within mid band
    railed = (Rm == 0) | (Gm == 0) | (Bm == 0) | (Rm == 255) | (Gm == 255) | (Bm == 255)
    honest_n = int((~railed).sum())
    honest_frac = float(railed.sum()) / nmid * 100.0
    honest_sat = float(sat[~railed].mean() * 100.0) if honest_n > 0 else float("nan")

    return {
        "label": _label(path),
        "size": "{}x{}".format(W, H),
        "rails": rails,
        "any0": any0,
        "any255": any255,
        "nmid": nmid,
        "mid_meanB": mid_meanB,
        "mid_B0": mid_B0,
        "mid_B_le2": mid_B_le2,
        "mid_any0": mid_any0,
        "cur_sat": cur_sat,
        "honest_sat": honest_sat,
        "honest_frac": honest_frac,
    }


def main():
    roots = sys.argv[1:] or ["_pixel_shotset_N6/before", "_pixel_shotset_N6/after"]
    files = []
    for r in roots:
        if os.path.isdir(r):
            files += sorted(glob.glob(os.path.join(r, "*-nohud2.png")))
        elif os.path.isfile(r):
            files.append(r)
    ref = "docs/assets/golden-beauty-shot-ref.png"
    if os.path.isfile(ref):
        files.append(ref)

    print("# gamut-clip probe -- {} frame(s) (native resolution)\n".format(len(files)))

    rows = [analyze(f) for f in files]

    # --- whole-frame rails: % pixels at 0 and 255, per channel ---
    print("{:<40}{:>6}{:>7}{:>6}{:>7}{:>6}{:>7}{:>8}{:>9}".format(
        "frame", "R=0", "R=255", "G=0", "G=255", "B=0", "B=255", "any0%", "any255%"))
    print("-" * 96)
    for d in rows:
        rl = d["rails"]
        print("{:<40}{:6.1f}{:7.1f}{:6.1f}{:7.1f}{:6.1f}{:7.1f}{:8.1f}{:9.1f}".format(
            d["label"], rl["R"][0], rl["R"][1], rl["G"][0], rl["G"][1],
            rl["B"][0], rl["B"][1], d["any0"], d["any255"]))

    # --- midtone band: the exact population grade_axes grades ---
    print("\n# midtone band L[p35..p75] (nmid pixels) -- what the gate actually sees")
    print("{:<40}{:>7}{:>8}{:>9}{:>10}{:>9}{:>11}{:>12}".format(
        "frame", "meanB", "B==0%", "B<=2%", "any==0%", "curSat%", "honestSat%", "railedFrac%"))
    print("-" * 106)
    for d in rows:
        hs = "{:11.1f}".format(d["honest_sat"]) if d["honest_sat"] == d["honest_sat"] else "{:>11}".format("nan")
        print("{:<40}{:7.2f}{:8.1f}{:9.1f}{:10.1f}{:9.1f}{}{:12.1f}".format(
            d["label"], d["mid_meanB"], d["mid_B0"], d["mid_B_le2"], d["mid_any0"],
            d["cur_sat"], hs, d["honest_frac"]))


if __name__ == "__main__":
    main()
