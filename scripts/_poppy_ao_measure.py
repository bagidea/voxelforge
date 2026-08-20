#!/usr/bin/env python3
"""Measure and photograph the voxel-AO A/B (same camera, same binary).

Reads the plate pair shot by scripts/_poppy_ao_shoot.ps1 and answers three
questions, in the order they can go wrong:

  1. Did anything move at all?  mean |delta|, share of pixels darkened.
     A dead lever and a working-but-subtle lever look the same in a caption;
     they do not look the same here.
  2. Did it move in the right DIRECTION?  AO may only ever darken. A frame
     where pixels brightened is not an AO frame, it is an exposure change.
  3. Did it move in the right PLACE?  Occlusion belongs at geometric edges,
     so the darkening is scored against a Sobel edge mask of the BEFORE
     frame: `edge_gain` is mean darkening near an edge divided by mean
     darkening in the flat interior. A flat multiply over the whole frame
     scores ~1.0; real per-vertex AO scores well above it.

Writes a side-by-side sheet and, for a named crop box, a zoomed corner pair.

USAGE
  python scripts/_poppy_ao_measure.py BEFORE.png AFTER.png OUTDIR
                                      [--crop X0,Y0,X1,Y1] [--label NAME]
"""

import argparse
import json
import os
import sys

import numpy as np
from PIL import Image, ImageDraw


def load(path):
    im = Image.open(path).convert("RGB")
    return im, np.asarray(im).astype(np.float32)


def luma(a):
    return a[..., 0] * 0.2126 + a[..., 1] * 0.7152 + a[..., 2] * 0.0722


def sobel(g):
    kx = np.array([[1, 0, -1], [2, 0, -2], [1, 0, -1]], dtype=np.float32)
    ky = kx.T
    p = np.pad(g, 1, mode="edge")
    out = np.zeros_like(g)
    gx = np.zeros_like(g)
    gy = np.zeros_like(g)
    for j in range(3):
        for i in range(3):
            w = p[j:j + g.shape[0], i:i + g.shape[1]]
            gx += kx[j, i] * w
            gy += ky[j, i] * w
    out = np.sqrt(gx * gx + gy * gy)
    return out


def dilate(mask, r):
    """Binary dilation by a (2r+1) square, via cumulative sums."""
    m = mask.astype(np.float32)
    p = np.pad(m, r, mode="constant")
    c = np.cumsum(np.cumsum(p, axis=0), axis=1)
    c = np.pad(c, ((1, 0), (1, 0)), mode="constant")
    k = 2 * r + 1
    h, w = m.shape
    box = (c[k:k + h, k:k + w] - c[0:h, k:k + w]
           - c[k:k + h, 0:w] + c[0:h, 0:w])
    return box > 0.5


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("before")
    ap.add_argument("after")
    ap.add_argument("outdir")
    ap.add_argument("--crop", default=None, help="X0,Y0,X1,Y1 zoom box")
    ap.add_argument("--label", default="ao")
    a = ap.parse_args()

    if not os.path.isfile(a.before) or not os.path.isfile(a.after):
        print("MISSING plate", a.before, a.after)
        return 2
    os.makedirs(a.outdir, exist_ok=True)

    bim, b = load(a.before)
    aim, f = load(a.after)
    if b.shape != f.shape:
        print("SHAPE MISMATCH", b.shape, f.shape)
        return 2

    lb, la = luma(b), luma(f)
    d = lb - la                       # positive = the AFTER frame is darker
    absd = np.abs(d)

    # 1. did anything move
    moved = float((absd > 2.0).mean())
    mean_abs = float(absd.mean())

    # 2. direction
    darker = float((d > 2.0).mean())
    brighter = float((d < -2.0).mean())

    # 3. placement: darkening near BEFORE-frame edges vs in flat interior.
    g = sobel(lb)
    edge = g > np.percentile(g, 88.0)
    near = dilate(edge, 3)
    flat = ~dilate(edge, 8)
    near_gain = float(d[near].mean()) if near.any() else 0.0
    flat_gain = float(d[flat].mean()) if flat.any() else 0.0
    # A null pair (nothing moved anywhere) would otherwise divide 0 by 0 and
    # report the same "inf" as a perfectly edge-localised change - the two
    # answers that most need telling apart. Controls: an identical pair scores
    # n/a, a global 0.9 multiply scores ~1.13, a synthetic edge-only darkening
    # scores inf.
    if mean_abs < 0.01:
        edge_gain = "n/a (null pair)"
    elif abs(flat_gain) > 1e-4:
        edge_gain = near_gain / flat_gain
    else:
        edge_gain = float("inf")

    res = {
        "before": a.before,
        "after": a.after,
        "mean_abs_delta": round(mean_abs, 4),
        "pct_pixels_moved": round(100 * moved, 3),
        "pct_darker": round(100 * darker, 3),
        "pct_brighter": round(100 * brighter, 3),
        "mean_luma_before": round(float(lb.mean()), 3),
        "mean_luma_after": round(float(la.mean()), 3),
        "darkening_near_edges": round(near_gain, 4),
        "darkening_in_flat": round(flat_gain, 4),
        "edge_gain": (round(edge_gain, 3) if isinstance(edge_gain, float)
                      and np.isfinite(edge_gain) else
                      edge_gain if isinstance(edge_gain, str) else "inf"),
    }
    print(json.dumps(res, indent=2))
    with open(os.path.join(a.outdir, f"{a.label}-metrics.json"), "w") as fh:
        json.dump(res, fh, indent=2)

    # ---- heatmap of the darkening (red = darker after) --------------------
    heat = np.zeros_like(b)
    amp = np.clip(d / max(1.0, np.percentile(absd, 99.5)), -1, 1)
    heat[..., 0] = np.clip(amp, 0, 1) * 255
    heat[..., 2] = np.clip(-amp, 0, 1) * 255
    Image.fromarray(heat.astype(np.uint8)).save(
        os.path.join(a.outdir, f"{a.label}-delta-heatmap.png"))

    # ---- side-by-side sheet ----------------------------------------------
    w, h = bim.size
    sheet = Image.new("RGB", (w * 2 + 24, h + 40), (18, 18, 20))
    sheet.paste(bim, (8, 32))
    sheet.paste(aim, (w + 16, 32))
    dr = ImageDraw.Draw(sheet)
    dr.text((10, 10), "BEFORE  VOXELFORGE_AO=off", fill=(235, 235, 235))
    dr.text((w + 18, 10), "AFTER  VOXELFORGE_AO=1 (corner + eave)", fill=(235, 235, 235))
    if a.crop:
        x0, y0, x1, y1 = (int(v) for v in a.crop.split(","))
        for ox in (8, w + 16):
            dr.rectangle([ox + x0, 32 + y0, ox + x1, 32 + y1], outline=(255, 210, 0))
    sheet.save(os.path.join(a.outdir, f"{a.label}-sheet.png"))

    # ---- zoom crop on one corner -----------------------------------------
    if a.crop:
        x0, y0, x1, y1 = (int(v) for v in a.crop.split(","))
        cw, ch = x1 - x0, y1 - y0
        z = max(1, min(6, 520 // max(1, cw)))
        cb = bim.crop((x0, y0, x1, y1)).resize((cw * z, ch * z), Image.NEAREST)
        ca = aim.crop((x0, y0, x1, y1)).resize((cw * z, ch * z), Image.NEAREST)
        zw, zh = cb.size
        pair = Image.new("RGB", (zw * 2 + 24, zh + 40), (18, 18, 20))
        pair.paste(cb, (8, 32))
        pair.paste(ca, (zw + 16, 32))
        dr = ImageDraw.Draw(pair)
        dr.text((10, 10), f"BEFORE  AO=off   crop {x0},{y0} {cw}x{ch} @{z}x",
                fill=(235, 235, 235))
        dr.text((zw + 18, 10), "AFTER  AO=1", fill=(235, 235, 235))
        pair.save(os.path.join(a.outdir, f"{a.label}-corner-zoom.png"))

        cbn, can = np.asarray(cb).astype(np.float32), np.asarray(ca).astype(np.float32)
        print("crop mean luma  before %.2f  after %.2f  delta %.2f" % (
            luma(cbn).mean(), luma(can).mean(),
            luma(cbn).mean() - luma(can).mean()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
