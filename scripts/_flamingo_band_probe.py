#!/usr/bin/env python3
"""Band-definition probe — input for Rose's re-derive of `blue` / `sat_honest`.

Question this answers, with numbers instead of an opinion:
  the gate's midtone band is `L in [p35, p75]` over the WHOLE frame. On an indoor
  golden ref that band is lit wood. On an outdoor plate it can be up to a third
  SKY -- and sky is the one thing in frame whose B is supposed to be high. So the
  `blue <= 10` target, calibrated indoors, is being asked of a band that contains
  the sky by construction.

For each plate at the shipped 1.02 rung it reports, on the same 1024 LANCZOS
geometry the gate uses:
  A) sky fraction inside the CURRENT band
  B) the four chromatic axes under the CURRENT band
  C) the same axes under a candidate TERRAIN band (sky dropped BEFORE percentiles)
and the same for the golden ref (0 % sky by construction -- the control).

Nothing here is a proposed target. It is the evidence that the current band is
measuring different content on different plates, plus one candidate replacement
Rose can accept, reject or refine.
"""
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PLATES = ["grade-vista", "hero", "s1-vista", "s4-raking"]
RUNG = "_yama_sat_sweep_102"
REF = os.path.join(ROOT, "docs", "assets", "golden-beauty-shot-ref.png")


def load(path):
    a = np.asarray(Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
                   ).astype(np.float32)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    return R, G, B, 0.2126 * R + 0.7152 * G + 0.0722 * B


def axes(R, G, B, mask):
    Rm, Gm, Bm = R[mask], G[mask], B[mask]
    railed = (Rm == 0) | (Gm == 0) | (Bm == 0) | (Rm == 255) | (Gm == 255) | (Bm == 255)
    clip = float(railed.sum()) / max(int(mask.sum()), 1) * 100.0
    mx = np.maximum(np.maximum(Rm, Gm), Bm)
    mn = np.minimum(np.minimum(Rm, Gm), Bm)
    hon = ~railed
    sat = (float(np.where(mx[hon] > 0, (mx[hon] - mn[hon]) / np.maximum(mx[hon], 1e-6), 0).mean() * 100)
           if int(hon.sum()) > 0 else float("nan"))
    return {"warmth": float((Rm - Bm).mean()), "blue": float(Bm.mean()),
            "clip": clip, "sat": sat, "n": int(mask.sum())}


def sky_of(R, G, B, L):
    """The diagnosis mask already used in the 2026-08-09 verdict: bluer than red AND
    brighter than the frame median. Crude but reproducible, and it is what the
    'TERRAIN space' numbers in that report were built on."""
    return (B > R) & (L > np.median(L))


def report(name, path):
    R, G, B, L = load(path)
    sky = sky_of(R, G, B, L)

    cur = (L >= np.percentile(L, 35)) & (L <= np.percentile(L, 75))          # CURRENT gate band
    ter = ~sky
    if ter.sum() > 1000:                                                     # CANDIDATE band
        lo, hi = np.percentile(L[ter], 35), np.percentile(L[ter], 75)
        terband = ter & (L >= lo) & (L <= hi)
    else:
        terband = cur

    a_cur, a_ter = axes(R, G, B, cur), axes(R, G, B, terband)
    sky_in_band = float(sky[cur].mean() * 100)
    return {
        "name": name,
        "sky_frame": float(sky.mean() * 100),
        "sky_in_current_band": sky_in_band,
        "cur": a_cur, "ter": a_ter,
    }


def main():
    rows = [report("REF golden (indoor)", REF)]
    for p in PLATES:
        rows.append(report(p, os.path.join(ROOT, RUNG, "after", f"{p}-nohud2.png")))

    out = []
    A = out.append
    A("### A) how much sky is inside the band the gate actually measures (rung 1.02)")
    A("")
    A("| plate | sky % of frame | **sky % INSIDE current band L[p35,p75]** | band px |")
    A("|---|---|---|---|")
    for r in rows:
        A(f"| {r['name']} | {r['sky_frame']:.1f} | **{r['sky_in_current_band']:.1f}** | {r['cur']['n']:,} |")
    A("")
    A("### B) current band vs C) candidate terrain band — same plates, same rung")
    A("")
    A("| plate | warmth cur -> ter | blue cur -> ter | clip cur -> ter | sat(honest) cur -> ter |")
    A("|---|---|---|---|---|")
    for r in rows:
        c, t = r["cur"], r["ter"]
        A(f"| {r['name']} | {c['warmth']:.1f} -> **{t['warmth']:.1f}** | "
          f"{c['blue']:.1f} -> **{t['blue']:.1f}** | {c['clip']:.1f} -> **{t['clip']:.1f}** | "
          f"{c['sat']:.1f} -> **{t['sat']:.1f}** |")
    A("")
    md = "\n".join(out)
    print(md)
    with open(os.path.join(ROOT, "_flamingo_band_probe.md"), "w", encoding="utf-8") as f:
        f.write(md + "\n")


if __name__ == "__main__":
    main()
