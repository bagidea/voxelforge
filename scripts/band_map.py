"""Flamingo — WHERE does the P0 midtone band actually live in a frame?

`scripts/grade_axes.py` measures warmth / blue / sat on "the midtone band":
every pixel whose luminance falls in [p35, p75] of the 1024x1024 resample.
On the golden INTERIOR reference that band is lit wood. On an outdoor vista
nobody has ever checked what it is — and you cannot pick a lever for an axis
until you know which pixels the axis is reading.

Emits, for one frame:
  * band share per horizontal 8th of the image (sky vs ground bias)
  * mean RGB of the band per 8th
  * a PNG mask so the band is visible, not just tabulated

Usage: python scripts/band_map.py FRAME.png [-o OUT_MASK.png]
"""

import argparse
import numpy as np
from PIL import Image

N = 1024


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("frame")
    ap.add_argument("-o", "--out", default=None)
    a = ap.parse_args()

    im = Image.open(a.frame).convert("RGB").resize((N, N), Image.LANCZOS)
    rgb = np.asarray(im).astype(np.float64)
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722])
    lo, hi = np.percentile(lum, 35), np.percentile(lum, 75)
    band = (lum >= lo) & (lum <= hi)

    print(f"{a.frame}")
    print(f"  band luminance window: {lo:.1f} .. {hi:.1f}   pixels {band.sum()}")
    print("  row-band   share%   meanR  meanG  meanB    R-B")
    step = N // 8
    for i in range(8):
        sl = band[i * step:(i + 1) * step]
        px = rgb[i * step:(i + 1) * step][sl]
        share = 100.0 * sl.sum() / max(1, band.sum())
        if px.size == 0:
            print(f"  y{i * step:4d}-{(i + 1) * step:4d}  {share:5.1f}      (empty)")
            continue
        m = px.mean(axis=0)
        print(
            f"  y{i * step:4d}-{(i + 1) * step:4d}  {share:5.1f}   "
            f"{m[0]:6.1f} {m[1]:6.1f} {m[2]:6.1f}  {m[0] - m[2]:6.1f}"
        )

    m = rgb[band].mean(axis=0)
    print(f"  WHOLE BAND      100.0   {m[0]:6.1f} {m[1]:6.1f} {m[2]:6.1f}  {m[0] - m[2]:6.1f}")

    out = a.out or a.frame.rsplit(".", 1)[0] + "_BAND.png"
    vis = rgb.copy()
    vis[~band] *= 0.18
    Image.fromarray(vis.astype(np.uint8)).save(out)
    print(f"  mask -> {out}")


if __name__ == "__main__":
    main()
