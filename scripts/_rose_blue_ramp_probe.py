#!/usr/bin/env python3
"""Blue-ramp + saturation evidence probe (Rose, 2026-08-09).

The live `blue B(mid) <= 10` gate is anti-correlated with quality (lower B means
more clamping means a worse frame) and scene-class-dominated (indoor wood 4.3 vs
outdoor grass/vista 16-91). This probe gathers the numbers to decide a
replacement statistic that is (a) quality-aligned and (b) scene-robust, and to
pick a threshold with evidence.

Candidates measured on EVERY plate (REF + N6 known-bad + sat-sweep good rungs):
  - midtone mean B            : the CURRENT statistic (scene-dominated, anti-Q)
  - corr(B, L)                : blue-ramp health; high = blue rises with light
  - top-5% (L>=p95) B / (B/R) : do highlights keep their blue, or bleach?
  - honest sat, clip, warmth  : the other live axes, for context

Each is computed in TWO bands:
  CUR  : whole-frame midtone L[p35,p75]            (what the gate uses today)
  TER  : terrain midtone  -- sky dropped, then L[p35,p75] on survivors (Task-2 fix)

REF has 0% sky so CUR==TER for it (the Task-2 invariant).

Nothing here is a target. It is the evidence for the re-derive.
"""
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REF = os.path.join(ROOT, "docs", "assets", "golden-beauty-shot-ref.png")
SWEEP_RUNGS = ["100", "102", "105", "115", "130", "190"]
SWEEP_PLATES = ["grade-vista", "hero", "s1-vista", "s4-raking"]
N6 = os.path.join(ROOT, "_pixel_shotset_N6", "after")


def load(path):
    a = np.asarray(Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
                   ).astype(np.float32)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    return R, G, B, 0.2126 * R + 0.7152 * G + 0.0722 * B


def corr(a, b):
    a = a.astype(np.float64); b = b.astype(np.float64)
    sa, sb = a.std(), b.std()
    if sa < 1e-6 or sb < 1e-6:
        return float("nan")
    return float(((a - a.mean()) * (b - b.mean())).mean() / (sa * sb))


def axes(R, G, B, L, mask):
    Rm, Bm, Lm = R[mask], B[mask], L[mask]
    railed = (Rm == 0) | (G[mask] == 0) | (Bm == 0) | (Rm == 255) | (G[mask] == 255) | (Bm == 255)
    clip = float(railed.sum()) / max(int(mask.sum()), 1) * 100.0
    mx = np.maximum(np.maximum(Rm, G[mask]), Bm)
    mn = np.minimum(np.minimum(Rm, G[mask]), Bm)
    hon = ~railed
    sat = (float(np.where(mx[hon] > 0, (mx[hon] - mn[hon]) / np.maximum(mx[hon], 1e-6), 0).mean() * 100)
           if int(hon.sum()) > 0 else float("nan"))
    # top-5% luminance slice of THIS mask: do highlights keep blue?
    hi = Lm >= np.percentile(Lm, 95)
    Bh, Rh = Bm[hi], Rm[hi]
    br_ratio = float(np.where(Rh > 1e-3, Bh / Rh, 0.0).mean()) if hi.sum() else float("nan")
    return {
        "n": int(mask.sum()),
        "warmth": float((Rm - Bm).mean()),
        "blue": float(Bm.mean()),
        "clip": clip,
        "sat": sat,
        "corrBL": corr(Bm, Lm),
        "hiB": float(Bh.mean()) if hi.sum() else float("nan"),
        "hiBR": br_ratio * 100.0,
    }


def report(name, path):
    R, G, B, L = load(path)
    sky = (B > R) & (L > np.median(L))
    ter = ~sky
    # CUR band: whole-frame percentiles
    cur = (L >= np.percentile(L, 35)) & (L <= np.percentile(L, 75))
    # TER band: terrain-only percentiles, then band on terrain
    if ter.sum() > 1000:
        lo, hi = np.percentile(L[ter], 35), np.percentile(L[ter], 75)
        terband = ter & (L >= lo) & (L <= hi)
    else:
        terband = cur
    return {
        "name": name,
        "sky_band": float(sky[cur].mean() * 100),
        "cur": axes(R, G, B, L, cur),
        "ter": axes(R, G, B, L, terband),
    }


def fmt(d, keys):
    return "  ".join(f"{k}={d[k]:7.2f}" for k in keys)


def main():
    rows = []
    rows.append(report("REF (indoor, good)", REF))
    # good outdoor plates at shipped 102 + recommended 105
    for plate in SWEEP_PLATES:
        for rung in ("102", "105"):
            p = os.path.join(ROOT, f"_yama_sat_sweep_{rung}", "after", f"{plate}-nohud2.png")
            if os.path.isfile(p):
                rows.append(report(f"{plate} @{rung} (good)", p))
    # N6 known-bad shotset (@1.90)
    import glob
    for p in sorted(glob.glob(os.path.join(N6, "*-nohud2.png"))):
        base = os.path.basename(p).replace("-nohud2.png", "")
        rows.append(report(f"{base} @190 (N6 bad)", p))

    keys = ["warmth", "blue", "clip", "sat", "corrBL", "hiBR"]
    out = []
    A = out.append
    A("# Blue-ramp + sat evidence (Rose probe, 2026-08-09)\n")
    A("REF must PASS, N6 (@190) must FAIL, good outdoor (@102/105) must PASS.\n")
    A("`corrBL` = corr(B,L) over the band (blue-ramp health). `hiBR` = top-5% L mean B/R %.\n")
    A("CUR = current whole-frame midtone band. TER = terrain band (sky dropped) -- REF unchanged.\n")
    A("\n## CUR band (what the gate uses today)\n")
    A("| plate | " + " | ".join(keys) + " | sky% |")
    A("|" + "---|" * (len(keys) + 2))
    for r in rows:
        A(f"| {r['name']} | " + " | ".join(f"{r['cur'][k]:.2f}" for k in keys) + f" | {r['sky_band']:.1f} |")
    A("\n## TER band (Task-2 candidate: sky dropped before percentiles)\n")
    A("| plate | " + " | ".join(keys) + " |")
    A("|" + "---|" * (len(keys) + 1))
    for r in rows:
        A(f"| {r['name']} | " + " | ".join(f"{r['ter'][k]:.2f}" for k in keys) + " |")
    txt = "\n".join(out)
    print(txt)
    with open(os.path.join(ROOT, "_rose_blue_ramp_probe.md"), "w", encoding="utf-8") as f:
        f.write(txt + "\n")


if __name__ == "__main__":
    main()
