#!/usr/bin/env python3
"""Sun — measure sky-vs-ground luma for a render, with an EXPOSURE-INVARIANT sky mask.

Reads pixels only. Writes nothing (unless --mask-out DIR). Never touches
.rs / assets / maps.

WHY THIS EXISTS
---------------
The sky-brighter-than-ground A/B (`_sun_skyground_shoot.ps1`) sweeps env levers
that change exposure / ambient / fill on the SAME camera + map + hour. The only
thing that is allowed to move between arms is the LIGHTING — the sky dome and
the horizon geometry are fixed. So `sky_frac_pct` (the area of the sky mask)
must be identical across all arms by construction.

It was not. The mask came from `_pixel_artgap_grade.sky_mask`, which thresholds
the RAW Sobel magnitude of Rec.709 luma (`sobel_mag(L) <= 22`). A Sobel
magnitude scales with exposure: darker frames have weaker edges, so the horizon
edge falls under the threshold and the flood-from-top leaks past it into the
ground. Result: sky_frac drifted 20.0% -> 34.1% from ev11.2 to ev11.8 while the
scene did not change, which also dragged `sky_ground_ratio` around because the
median sky/ground split depends on the same mask.

THE FIX
-------
The flood-fill geometry is kept (sky = region reachable from the top without
crossing a hard edge) — only the edge test is made exposure-invariant. Instead
of thresholding the absolute Sobel magnitude, we threshold the CONTRAST-
NORMALISED (relative) Sobel magnitude:

    rel_gradient = sobel_mag(L) / (gaussian_filter(L, 2.0) + eps)

Under a multiplicative exposure change L -> k*L both numerator and denominator
scale by k, so `rel_gradient` is invariant to exposure (and approximately
invariant to the tonemap toe, which is the residual). The threshold
`SKY_T_REL = 0.12` was calibrated on the ev11.2/ev11.8/amb420/amb300/fill800
set: it is the largest threshold before the darkest arm (ev11.8) starts to leak
(leak onset at 0.14).

This file deliberately does NOT import `sky_mask` from `_pixel_artgap_grade`
(that helper is Flamingo's lane and still has the exposure-dependent threshold);
it imports only the pure, stable helpers `load` / `luma` / `sobel_mag`.

Usage:
    python scripts/_sun_skyground_measure.py <frame.png> [frame2.png ...] [--mask-out DIR]
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _pixel_artgap_grade import load, luma, sobel_mag  # noqa: E402  (pure helpers only)

# Contrast-normalised Sobel magnitude that counts as a HARD (block/geometry)
# edge. Same flood-fill semantics as the grader's sky_mask, but the edge test is
# exposure-invariant (relative gradient, not absolute luma gradient).
SKY_T_REL = 0.12


def sky_mask_invariant(L: np.ndarray, t_rel: float = SKY_T_REL) -> np.ndarray:
    """Region reachable from the top edge without crossing a hard edge, where
    "hard" is a CONTRAST-NORMALISED (exposure-invariant) Sobel magnitude.

    Contrast-normalising makes the edge test invariant to a multiplicative
    exposure change (L -> k*L), so darkening the frame does not turn real
    geometry edges into passable "smooth" pixels and let the sky leak into the
    ground. Deliberately brightness-blind in the same way as the grader's mask:
    a sky rendered as a dark void still comes back as sky.
    """
    denom = ndimage.gaussian_filter(L, 2.0) + 1e-6
    rel = sobel_mag(L) / denom
    free = rel <= t_rel
    lab, n = ndimage.label(free, structure=np.array([[0, 1, 0], [1, 1, 1], [0, 1, 0]]))
    if n == 0:
        return np.zeros_like(L, dtype=bool)
    top = np.unique(lab[0, :])
    top = top[top > 0]
    if top.size == 0:
        return np.zeros_like(L, dtype=bool)
    sky = np.isin(lab, top)
    # border_value=1: the default erodes the top rows away and silently zeroed
    # every horizon on the grader (see its sky_mask docstring) — keep parity.
    sky = ndimage.binary_closing(sky, np.ones((5, 5)), border_value=1)
    # a sky is a lid, not a curtain: drop columns where the region reaches the
    # bottom of the frame (that is a wall / backdrop running the full height).
    H, W = L.shape
    through = sky[H - 1, :]
    if through.any():
        sky[:, through] = False
    return sky


def _measure(path: str) -> dict:
    rgb = load(path)
    L = luma(rgb)
    sky = sky_mask_invariant(L)

    npx = L.size
    sky_frac = float(sky.mean() * 100)

    if sky.sum() >= 200:
        Ls = L[sky]
        sky_med = float(np.median(Ls))
        sky_L_mean = float(Ls.mean())
        sky_void_pct = float((Ls < 10).mean() * 100)
        sky_blown_pct = float((Ls > 245).mean() * 100)
        gnd = L[~sky]
        gnd_med = float(np.median(gnd)) if gnd.size else None
        sky_ground_ratio = (round(sky_med / gnd_med, 3)
                            if gnd_med and gnd_med > 1e-6 else None)
    else:
        sky_med = sky_L_mean = sky_void_pct = sky_blown_pct = None
        gnd_med = None
        sky_ground_ratio = None

    return {
        "file": path,
        "px": f"{L.shape[1]}x{L.shape[0]}",
        "sky_frac_pct": round(sky_frac, 2),
        "sky_median_L": round(sky_med, 2) if sky_med is not None else None,
        "ground_median_L": round(gnd_med, 2) if gnd_med is not None else None,
        "sky_L_mean": round(sky_L_mean, 2) if sky_L_mean is not None else None,
        "sky_ground_ratio": sky_ground_ratio,
        "sky_void_pct": round(sky_void_pct, 2) if sky_void_pct is not None else None,
        "sky_blown_pct": round(sky_blown_pct, 2) if sky_blown_pct is not None else None,
    }


def _dump_mask(path: str, out: str) -> None:
    from PIL import Image
    rgb = load(path)
    L = luma(rgb)
    sky = sky_mask_invariant(L)
    v = rgb.copy()
    v[sky] = v[sky] * 0.35 + np.array([0, 120, 255]) * 0.65
    H, W, _ = v.shape
    hz = np.where(sky.any(axis=0), np.argmin(sky, axis=0), 0)
    hz = np.where(sky[0, :], hz, 0)
    for x in range(W):
        v[min(H - 1, hz[x]), x] = [255, 255, 0]
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(v.clip(0, 255).astype(np.uint8)).save(out)


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--mask-out=")]
    mask_dir = None
    for a in sys.argv[1:]:
        if a.startswith("--mask-out="):
            mask_dir = a.split("=", 1)[1]

    for path in args:
        m = _measure(path)
        if mask_dir:
            _dump_mask(path, str(Path(mask_dir) / (Path(path).stem + "_mask.png")))
        print(json.dumps(m))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
