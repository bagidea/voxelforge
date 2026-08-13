#!/usr/bin/env python3
"""A6.2 metric bake-off — find a `hero cuts against the background by COLOUR`
statistic that the approved art passes and the acknowledged failure frame fails.

Why this file exists: the shipped gate (`art_order_grade.py` A6
`hero vs bg hue split >= 25 deg`) is a whole-blob LINEAR MEAN of hue.  Run on the
three CEO-approved concept sheets it returns 1.4 / 1.4 / 5.8 deg — the approved
art fails its own bar, which makes 25 a phantom target of exactly the kind
`docs/look-acceptance-rubric.md` already retired once (P0-axes, 2026-07-26).
See `_flamingo_a62_huesep_control.py` for that control run.

This script measures candidate replacements on the same two populations:

  POSITIVE (must pass) : the three approved concept sheets — this is the look
                         the CEO signed off, so any gate it fails is wrong.
  NEGATIVE (must fail) : the pre-accent gate3 frames the art order itself
                         diagnosed as "hero is an orange box in an orange world".

A statistic only earns a threshold if it separates those two populations.
Everything is measured on the same hero mask + background ring the shipped gate
uses, so the comparison isolates the STATISTIC, not the mask.
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
import art_order_grade as A  # noqa: E402
import grade_character as C  # noqa: E402


def ring_of(blob):
    return (ndimage.binary_dilation(blob, iterations=24)
            & ~ndimage.binary_dilation(blob, iterations=10))


def lab_of(rgb):
    """sRGB(0-255) -> CIELAB, D65. Vectorised, no colour library needed."""
    s = rgb / 255.0
    lin = np.where(s <= 0.04045, s / 12.92, ((s + 0.055) / 1.055) ** 2.4)
    m = np.array([[0.4124, 0.3576, 0.1805],
                  [0.2126, 0.7152, 0.0722],
                  [0.0193, 0.1192, 0.9505]], dtype=np.float32)
    xyz = lin @ m.T
    wp = np.array([0.95047, 1.0, 1.08883], dtype=np.float32)
    t = xyz / wp
    d = 6.0 / 29.0
    fx = np.where(t > d ** 3, np.cbrt(t), t / (3 * d * d) + 4.0 / 29.0)
    L = 116 * fx[..., 1] - 16
    a = 500 * (fx[..., 0] - fx[..., 1])
    b = 200 * (fx[..., 1] - fx[..., 2])
    return L, a, b


def hue_dist(h, ref):
    d = np.abs(h - ref) % 360.0
    return np.minimum(d, 360.0 - d)


def stats(path, blob):
    f = A.load(path)
    rgb, hue, sat = f["rgb"], f["hue"], f["sat"]
    ring = ring_of(blob)
    L, a, b = lab_of(rgb)
    out = {}

    # --- the shipped gate, for reference -----------------------------------
    hh, bh = float(hue[blob].mean()), float(hue[ring].mean())
    d = abs(hh - bh) % 360.0
    out["hue_sep(mean)"] = min(d, 360.0 - d)

    # --- chroma-weighted circular hue mean ----------------------------------
    # a large desaturated warm mass drowns any accent in a plain mean; weighting
    # by chroma and averaging on the circle is the textbook repair.
    def cwhue(m):
        w = np.hypot(a[m], b[m])
        ang = np.deg2rad(hue[m])
        return np.rad2deg(np.arctan2((w * np.sin(ang)).sum(), (w * np.cos(ang)).sum())) % 360.0
    d = abs(cwhue(blob) - cwhue(ring)) % 360.0
    out["hue_sep(chroma-wt)"] = min(d, 360.0 - d)

    # --- chroma-only Lab distance (ignores the value separation A6 says we
    #     already have too much of) -------------------------------------------
    out["dE_ab(chroma)"] = float(np.hypot(a[blob].mean() - a[ring].mean(),
                                          b[blob].mean() - b[ring].mean()))

    # --- accent-cut: an accent is a LOCAL cut, not a shift of the mean --------
    bgh = bh
    for lim in (45, 60, 90):
        cut = (hue_dist(hue[blob], bgh) >= lim) & (sat[blob] >= 0.25)
        out[f"cut>={lim}deg %hero"] = 100.0 * cut.mean()

    # --- hue spread inside the hero (does the body carry >1 hue family?) -----
    out["hero hue IQR"] = float(np.subtract(*np.percentile(hue[blob], [75, 25])))
    out["hero chroma p95"] = float(np.percentile(np.hypot(a[blob], b[blob]), 95))
    out["hero px"] = int(blob.sum())
    return out


def blob_for_concept(path):
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
    return C.mask_from_ref(rgb)


def blob_for_frame(frame, charmask):
    fr = np.asarray(Image.open(frame).convert("RGB"), dtype=np.int16)
    ov = np.asarray(Image.open(charmask).convert("RGB"), dtype=np.int16)
    return np.abs(fr - ov).sum(axis=2) > 20


def main():
    rows = []
    for c in ("auren-hero", "elder-maren", "guard-husk"):
        p = f"docs/assets/characters/{c}-concept.png"
        rows.append(("POS approved concept", c, stats(p, blob_for_concept(p))))
    for tag in ("boot", "walk", "combat"):
        d = Path("docs/assets/gate3-a6-2026-08-10")
        fr, cm = d / f"gate3-after-{tag}-nohud2.png", d / f"gate3-after-{tag}-nohud2-charmask.png"
        if fr.exists() and cm.exists():
            rows.append(("NEG pre-accent gate3", tag, stats(str(fr), blob_for_frame(fr, cm))))

    keys = [k for k in rows[0][2] if k != "hero px"]
    w = max(len(k) for k in keys) + 2
    print(f"{'population':22s} {'plate':13s} " + " ".join(f"{k:>18s}" for k in keys))
    print("-" * (22 + 14 + 19 * len(keys)))
    for pop, name, s in rows:
        print(f"{pop:22s} {name:13s} " + " ".join(f"{s[k]:18.2f}" for k in keys))

    print("\nseparation (min POS  vs  max NEG) — a gate is only honest where POS > NEG:")
    for k in keys:
        pos = [s[k] for p, _, s in rows if p.startswith("POS")]
        neg = [s[k] for p, _, s in rows if p.startswith("NEG")]
        if not neg:
            continue
        gap = min(pos) - max(neg)
        mark = "SEPARATES" if gap > 0 else "no separation"
        print(f"  {k:<{w}s} POS min {min(pos):8.2f}   NEG max {max(neg):8.2f}   "
              f"gap {gap:+8.2f}   {mark}")

    Path("docs/assets/_flamingo_a62_metric_probe.json").write_text(
        json.dumps([{"pop": p, "plate": n, **{k: float(v) for k, v in s.items()}} for p, n, s in rows], indent=1))


if __name__ == "__main__":
    main()
