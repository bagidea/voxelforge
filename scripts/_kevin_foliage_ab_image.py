#!/usr/bin/env python3
"""Kevin — build the before|after comparison plate for the foliage density pass.

Side-by-side stills + a caption bar labelling edge_mean, strong% and FPS for each
side, plus the foliage delta and the run-to-run noise floor (before vs before2,
same map, two runs) so the delta is read against the exe's own non-determinism.
FPS is marked N/A: the 14-Aug target-kevin/perf exe predates the
VOXELFORGE_FPS_BENCH sampler (in source, not yet built) — no frame-time stdout.

Usage:  python scripts/_kevin_foliage_ab_image.py
"""
import os

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FOL = os.path.join(ROOT, "_kevin_foliage")
OUT = os.path.join(ROOT, "docs", "assets", "look", "kevin-foliage-ab-2026-08-18.png")


def edge_strong(path):
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    L = 0.299 * a[..., 0] + 0.587 * a[..., 1] + 0.114 * a[..., 2]
    gray = ndimage.gaussian_filter(L, sigma=0.8)
    gx = ndimage.sobel(gray, axis=0)
    gy = ndimage.sobel(gray, axis=1)
    mag = np.hypot(gx, gy)
    return float(mag.mean()), float((mag > 40).mean())


def _font(size):
    for p in (r"C:\Windows\Fonts\arialbd.ttf", r"C:\Windows\Fonts\arial.ttf"):
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()


def main():
    em_b, st_b = edge_strong(os.path.join(FOL, "before.png"))
    em_a, st_a = edge_strong(os.path.join(FOL, "after.png"))
    em_b2, st_b2 = edge_strong(os.path.join(FOL, "before2.png"))

    img_b = Image.open(os.path.join(FOL, "before.png")).convert("RGB")
    img_a = Image.open(os.path.join(FOL, "after.png")).convert("RGB")
    if img_a.size != img_b.size:
        img_a = img_a.resize(img_b.size, Image.LANCZOS)

    W, H = img_b.size
    BAR = 96
    gap = 4
    canvas = Image.new("RGB", (W * 2 + gap, H + BAR), (10, 12, 16))
    canvas.paste(img_b, (0, 0))
    canvas.paste(img_a, (W + gap, 0))
    d = ImageDraw.Draw(canvas)
    big = _font(26)
    small = _font(17)
    title = _font(22)

    fps = "FPS N/A (sampler unbuilt in 14-Aug exe)"

    def side(x0, name, em, st):
        d.text((x0 + 10, H + 8), f"{name}  edge_mean {em:.2f}   strong {st*100:.1f}%",
               fill=(235, 238, 242), font=big)
        d.text((x0 + 10, H + 46), fps, fill=(150, 158, 168), font=small)

    side(0, "BEFORE (9537 blk)", em_b, st_b)
    side(W + gap, "AFTER (11063 blk)", em_a, st_a)

    de = em_a - em_b
    ds = (st_a - st_b) * 100
    noise_em = abs(em_b2 - em_b)
    noise_st = abs(st_b2 - st_b) * 100
    d.text((W - 310, H + 8),
           f"delta  edge {de:+.2f}   strong {ds:+.1f} pp",
           fill=(250, 214, 120), font=title)
    d.text((W - 310, H + 44),
           f"noise floor  edge {noise_em:.2f}   strong {noise_st:.1f} pp",
           fill=(180, 120, 130), font=small)

    d.rectangle([0, H - 1, W * 2 + gap, H + 1], fill=(56, 60, 68))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    canvas.save(OUT)
    print("saved", OUT, canvas.size)
    print(f"before  edge {em_b:.2f} strong {st_b*100:.1f}%")
    print(f"before2 edge {em_b2:.2f} strong {st_b2*100:.1f}%")
    print(f"after   edge {em_a:.2f} strong {st_a*100:.1f}%")
    print(f"delta   edge {de:+.2f} strong {ds:+.1f} pp   |   noise edge {noise_em:.2f} strong {noise_st:.1f} pp")


if __name__ == "__main__":
    main()
