#!/usr/bin/env python3
"""Shino — INDEPENDENT re-derivation of Sun's exposure-invariant sky mask.

Imports NOTHING from Sun's script or from _pixel_artgap_grade. Own PNG load,
own Rec.709 luma, own Sobel, own flood fill (BFS, not scipy.label), own
box-blur denominator instead of scipy gaussian_filter. If the numbers land on
Sun's table anyway, the result is real and not an artifact of one code path.

Also re-runs the OLD absolute-threshold rule (sobel <= 22) to confirm the
before-column drift Sun reported.
"""
import json
import sys
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image


def my_load(p):
    im = Image.open(p).convert("RGB")
    return np.asarray(im).astype(np.float64)


def my_luma(rgb):
    return 0.2126 * rgb[:, :, 0] + 0.7152 * rgb[:, :, 1] + 0.0722 * rgb[:, :, 2]


def my_sobel(L):
    kx = np.array([[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]], dtype=np.float64)
    ky = kx.T
    P = np.pad(L, 1, mode="edge")
    gx = np.zeros_like(L)
    gy = np.zeros_like(L)
    for dy in range(3):
        for dx in range(3):
            w = P[dy:dy + L.shape[0], dx:dx + L.shape[1]]
            gx += kx[dy, dx] * w
            gy += ky[dy, dx] * w
    return np.hypot(gx, gy)


def my_boxblur(L, r=3):
    """Cheap local-mean, deliberately NOT the gaussian Sun used."""
    P = np.pad(L, r, mode="edge")
    C = np.cumsum(np.cumsum(P, axis=0), axis=1)
    C = np.pad(C, ((1, 0), (1, 0)))
    k = 2 * r + 1
    H, W = L.shape
    tot = (C[k:k + H, k:k + W] - C[0:H, k:k + W]
           - C[k:k + H, 0:W] + C[0:H, 0:W])
    return tot / (k * k)


def my_flood_from_top(free):
    """BFS from every free pixel on row 0 (no scipy.label)."""
    H, W = free.shape
    seen = np.zeros_like(free, dtype=bool)
    q = deque()
    for x in range(W):
        if free[0, x]:
            seen[0, x] = True
            q.append((0, x))
    while q:
        y, x = q.popleft()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < H and 0 <= nx < W and free[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                q.append((ny, nx))
    return seen


def drop_curtains(sky):
    """A sky is a lid, not a curtain — same semantic, my own code."""
    H, W = sky.shape
    through = sky[H - 1, :]
    if through.any():
        sky = sky.copy()
        sky[:, through] = False
    return sky


def measure(path, mode):
    rgb = my_load(path)
    L = my_luma(rgb)
    s = my_sobel(L)
    if mode == "rel":
        free = (s / (my_boxblur(L, 3) + 1e-6)) <= 0.12
    else:                       # the OLD rule Sun says was broken
        free = s <= 22.0
    sky = drop_curtains(my_flood_from_top(free))
    out = {"file": Path(path).stem, "mode": mode,
           "px": f"{L.shape[1]}x{L.shape[0]}",
           "sky_frac_pct": round(float(sky.mean() * 100), 2)}
    if sky.sum() >= 200:
        gnd = L[~sky]
        sm = float(np.median(L[sky]))
        gm = float(np.median(gnd)) if gnd.size else None
        out["sky_median_L"] = round(sm, 2)
        out["ground_median_L"] = round(gm, 2) if gm else None
        out["sky_ground_ratio"] = round(sm / gm, 3) if gm and gm > 1e-6 else None
        # horizon = lowest sky row per column, median over columns that have sky
        cols = [int(np.max(np.nonzero(sky[:, x])[0])) for x in range(sky.shape[1])
                if sky[:, x].any()]
        out["horizon_median_row"] = int(np.median(cols)) if cols else None
    return out, sky


def main():
    paths = sys.argv[1:]
    masks = {}
    rows = []
    for mode in ("abs", "rel"):
        for p in paths:
            m, sk = measure(p, mode)
            rows.append(m)
            masks[(mode, Path(p).stem)] = sk
            print(json.dumps(m), flush=True)
    # cross-arm mask XOR under the new rule
    stems = [Path(p).stem for p in paths]
    base = masks[("rel", stems[0])]
    for s in stems[1:]:
        x = np.logical_xor(base, masks[("rel", s)]).mean() * 100
        print(json.dumps({"xor_vs_" + stems[0]: s, "pct": round(float(x), 3)}))
    for mode in ("abs", "rel"):
        v = [r["sky_frac_pct"] for r in rows if r["mode"] == mode]
        print(json.dumps({"mode": mode, "spread_sky_frac_pct": round(max(v) - min(v), 2)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
