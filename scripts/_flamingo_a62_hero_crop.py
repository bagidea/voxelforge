#!/usr/bin/env python3
"""Crop + upscale the hero out of a gate3 plate so the clasp region can be READ.

Three z re-derivations have now been argued from geometry alone and all three
shot 0 px. Before a fourth, look at the actual pixels: locate the character from
the mask `grade_character.py` already wrote, crop it, and blow it up nearest-
neighbour so voxel faces stay square and a 7-px gem would be unmistakable.

Also prints, per extra_part colour, how many pixels in the hero mask sit within a
tolerance of that colour's HUE — the point being to answer "does ANY extra_part
render?" (hair / ember pouch) separately from "does the clasp render?". If the
hair is missing too, the defect is upstream of the clasp's z entirely.

Usage:
    python scripts/_flamingo_a62_hero_crop.py FRAME [--scale 6] [--out DIR]
"""
import argparse
import colorsys
from pathlib import Path

import numpy as np
from PIL import Image

# The Player arm of anim.rs `extra_parts`, in declaration order.
EXTRA_COLOURS = [
    ("hair crown/nape/clumps (x4)", (0.165, 0.106, 0.071)),
    ("pouch housing #C88A4A", (0.78, 0.54, 0.29)),
    ("pouch coal #FFD98A", (1.0, 0.85, 0.54)),
    ("clasp gem #4FC9D6", (0.310, 0.788, 0.839)),
]


def hue_of(rgb01):
    return colorsys.rgb_to_hsv(*rgb01)[0] * 360.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("frame")
    ap.add_argument("--scale", type=int, default=6)
    ap.add_argument("--pad", type=int, default=14)
    ap.add_argument("--out", default="_fl_a62_control")
    args = ap.parse_args()

    frame = Path(args.frame)
    cm = frame.with_suffix("").as_posix() + "-charmask.png"
    a = np.asarray(Image.open(frame).convert("RGB"), dtype=np.int16)

    if Path(cm).exists():
        b = np.asarray(Image.open(cm).convert("RGB"), dtype=np.int16)
        mask = np.abs(a - b).sum(axis=2) > 20
    else:
        raise SystemExit(f"no charmask next to {frame} — run grade_character.py first")

    ys, xs = np.nonzero(mask)
    x0, x1 = max(0, xs.min() - args.pad), min(a.shape[1], xs.max() + args.pad)
    y0, y1 = max(0, ys.min() - args.pad), min(a.shape[0], ys.max() + args.pad)
    print(f"hero mask {mask.sum():,} px   bbox x[{x0},{x1}] y[{y0},{y1}]  "
          f"({x1-x0}x{y1-y0})")

    crop = Image.open(frame).convert("RGB").crop((x0, y0, x1, y1))
    big = crop.resize((crop.width * args.scale, crop.height * args.scale),
                      Image.NEAREST)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    dst = out / f"{frame.stem}-hero{args.scale}x.png"
    big.save(dst)
    print(f"wrote {dst}  ({big.width}x{big.height})")

    # --- which extra_parts are on screen at all? -----------------------------
    f = a.astype(np.float32)
    mx, mn = f.max(2), f.min(2)
    d = np.maximum(mx - mn, 1e-6)
    r, g, bl = f[..., 0], f[..., 1], f[..., 2]
    hue = np.where(mx == r, ((g - bl) / d) % 6,
                   np.where(mx == g, (bl - r) / d + 2, (r - g) / d + 4)) * 60.0
    hue %= 360.0
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    L = 0.2126 * r + 0.7152 * g + 0.0722 * bl

    print("\nextra_part presence by hue, INSIDE the hero mask "
          "(sat>=0.15, L>=12 — loose on purpose):")
    for label, c in EXTRA_COLOURS:
        h = hue_of(c)
        dh = np.abs(hue - h) % 360.0
        dh = np.minimum(dh, 360.0 - dh)
        hit = (dh <= 20.0) & (sat >= 0.15) & (L >= 12.0) & mask
        print(f"  {label:32s} hue {h:6.1f}   {int(hit.sum()):6d} px in mask")


if __name__ == "__main__":
    main()
