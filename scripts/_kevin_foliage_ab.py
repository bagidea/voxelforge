#!/usr/bin/env python3
"""Kevin — foliage-density pass: build the before|after comparison image.

Reads the two stills, re-measures edge/strong with the SAME pinned `measure()`
as _kevin_foliage_measure.py, and renders a side-by-side PNG labelled with
edge_mean + strong% + delta for each leg. FPS is labelled N/A because the 14-Aug
target-kevin/perf exe predates the VOXELFORGE_FPS_BENCH sampler.

Usage:  python scripts/_kevin_foliage_ab.py [before.png] [after.png] [out.png]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _kevin_art_gap_measure import measure  # noqa: E402

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DEFAULT = os.path.join(ROOT, "docs", "assets", "look", "kevin-foliage-ab-2026-08-18.png")


def _font(size):
    for p in (r"C:\Windows\Fonts\segoeui.ttf",
              r"C:\Windows\Fonts\segoeuib.ttf",
              r"C:\Windows\Fonts\arial.ttf",
              r"C:\Windows\Fonts\calibri.ttf"):
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                continue
    return ImageFont.load_default()


def main():
    before = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "_kevin_foliage", "before.png")
    after = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "_kevin_foliage", "after.png")
    out = sys.argv[3] if len(sys.argv) > 3 else OUT_DEFAULT

    b = measure(before, "before")
    a = measure(after, "after")
    d_edge = a["edge_mean"] - b["edge_mean"]
    d_strong = a["strong"] - b["strong"]

    im_b = Image.open(before).convert("RGB")
    im_a = Image.open(after).convert("RGB")

    # Common height for the pair.
    H = 720
    def scaled(im):
        w = int(im.width * H / im.height)
        return im.resize((w, H), Image.LANCZOS)

    sb = scaled(im_b)
    sa = scaled(im_a)

    PAD = 8
    LABEL_H = 48
    FOOT_H = 88
    W = sb.width + sa.width + PAD * 3
    canvas = Image.new("RGB", (W, LABEL_H + H + FOOT_H), (18, 18, 22))
    dr = ImageDraw.Draw(canvas)
    f_lab = _font(30)
    f_foot = _font(24)

    canvas.paste(sb, (PAD, LABEL_H))
    canvas.paste(sa, (PAD * 2 + sb.width, LABEL_H))

    dr.text((PAD, 8), "BEFORE  (9537 blocks)", fill=(255, 255, 255), font=f_lab)
    dr.text((PAD * 2 + sb.width, 8), "AFTER  (11063 blocks)", fill=(255, 255, 255), font=f_lab)

    fy = LABEL_H + H + 12
    dr.text((PAD, fy),
            "edge_mean: %.2f -> %.2f   (delta %+.2f)" % (b["edge_mean"], a["edge_mean"], d_edge),
            fill=(200, 220, 255), font=f_foot)
    dr.text((PAD, fy + 30),
            "strong:  %.2f%% -> %.2f%%   (delta %+.1f pp)" % (
                b["strong"] * 100, a["strong"] * 100, d_strong * 100),
            fill=(200, 220, 255), font=f_foot)
    dr.text((PAD, fy + 60),
            "FPS: N/A (14-Aug exe predates VOXELFORGE_FPS_BENCH sampler — relative delta only)",
            fill=(150, 150, 160), font=f_foot)

    canvas.save(out)
    print("WROTE %s" % out)
    print("edge_mean delta = %+.2f   strong delta = %+.4f (%+.1f pp)" % (d_edge, d_strong, d_strong * 100))


if __name__ == "__main__":
    main()
