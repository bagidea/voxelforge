#!/usr/bin/env python3
"""Poppy -- turn the A3 cloud-deck bake dump into a viewable PNG.

`VOXELFORGE_CLOUDS_DUMP=<path>` makes `look.rs::bake_cloud_deck` write the raw
RGBA8 bytes it is about to hand the GPU. This script is the only thing that
reads them, and it deliberately does NOT re-derive a single cloud value: it is a
byte reformatter, so a preview can never be of anything but what the frame will
sample. (The scar this avoids: a python "mirror" of shader maths that drifts
from the shader and then gets trusted.)

Two panels, stacked, so the two halves of the bake are separable by eye:

  top     RGB with alpha composited over a mid-grey -- what the deck paints
  bottom  alpha alone, as greyscale -- where the deck is opaque

Rows are elevation with row 0 at the HORIZON (that is the bake's own order), so
the image reads upside down compared to the sky. `--flip` puts the zenith on top
if you would rather look at it the way it renders.

Usage:
  _poppy_clouds_preview.py <dump.rgba> <W> <H> <out.png> [--flip]
"""
import sys

import numpy as np
from PIL import Image


def main(argv):
    if len(argv) < 5:
        print(__doc__)
        return 2
    src, w, h, out = argv[1], int(argv[2]), int(argv[3]), argv[4]
    flip = "--flip" in argv[5:]

    raw = np.fromfile(src, dtype=np.uint8)
    want = w * h * 4
    if raw.size != want:
        print(f"FAIL {src}: {raw.size} bytes, expected {want} for {w}x{h} RGBA8")
        return 1
    px = raw.reshape(h, w, 4)

    rgb = px[:, :, :3].astype(np.float32)
    a = (px[:, :, 3:4].astype(np.float32)) / 255.0
    # Composite over mid-grey so a bright cloud at low alpha stays legible and a
    # dark cloud at low alpha does not vanish into black.
    over = rgb * a + 128.0 * (1.0 - a)

    top = over.clip(0, 255).astype(np.uint8)
    bot = np.repeat((a * 255.0).astype(np.uint8), 3, axis=2)
    sheet = np.concatenate([top, bot], axis=0)
    if flip:
        sheet = np.concatenate([top[::-1], bot[::-1]], axis=0)

    Image.fromarray(sheet, "RGB").save(out)

    print(f"{src} -> {out}  {w}x{h}")
    print(f"  alpha   mean {a.mean():.4f}  p50 {np.median(a):.4f}  "
          f"p95 {np.percentile(a, 95):.4f}  share>0.5 {(a > 0.5).mean()*100:.1f}%")
    for i, name in enumerate("RGB"):
        c = rgb[:, :, i]
        print(f"  {name} (stored, 0-255)  mean {c.mean():6.2f}  p95 {np.percentile(c, 95):6.2f}"
              f"  max {c.max():3.0f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
