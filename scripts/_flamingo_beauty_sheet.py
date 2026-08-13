#!/usr/bin/env python3
"""Contact sheet for the 2026-08-14 beauty gap — reference vs current, and the
same two frames POSTERISED to 6 luminance bands.

The posterised row is the argument. A frame that spends a wide value range breaks
into readable bands (sky / lit / mid / shade / dark); a flat frame collapses into
one or two. It shows the defect the CEO named ("one orange tone, no near-far") as
a picture rather than as a number, and it does it without a chart axis to argue
with — the bands ARE the histogram.

Usage:
    python scripts/_flamingo_beauty_sheet.py OUT.png
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))

REF = ("docs/assets/moodboard.png", (514, 0, 1024, 1024),
       "REFERENCE: moodboard outdoor panel (approved)")
CUR = ("docs/assets/gate3/sky-dome.png", (0, 30, 2560, 1300),
       "CURRENT: gate3 sky-dome.png (shipped build)")
BANDS = 6
# One ramp for BOTH images so a band means the same luminance in each — what is
# compared is the SHARE of the frame each band takes, not how many are touched.
EDGES = np.linspace(0, 255, BANDS + 1)
RAMP = [(28, 22, 30), (72, 52, 44), (120, 82, 52), (172, 120, 66),
        (214, 168, 104), (252, 232, 200)]


def prep(path, crop, w, h):
    im = Image.open(path).convert("RGB").crop(crop)
    return im.resize((w, h), Image.LANCZOS)


def posterise(im):
    a = np.asarray(im, dtype=np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    idx = np.clip(np.digitize(L, EDGES[1:-1]), 0, BANDS - 1)
    out = np.array(RAMP, dtype=np.uint8)[idx]
    # NOT "how many bands are occupied" — both frames touch all six, so that
    # counter says nothing. What separates them is how much of the frame piles
    # into the two biggest bands.
    share = np.bincount(idx.ravel(), minlength=BANDS) / idx.size
    top2 = float(np.sort(share)[::-1][:2].sum())
    return Image.fromarray(out), top2


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "_fl_beauty/beauty-gap.png"
    cw, ch = 900, 520
    pad, top, gap = 24, 64, 44
    W = pad * 2 + cw * 2 + pad
    H = top + ch + gap + ch + 90
    sheet = Image.new("RGB", (W, H), (18, 18, 22))
    d = ImageDraw.Draw(sheet)
    d.text((pad, 18), "VOXELFORGE — beauty gap 2026-08-14   (top: frames, bottom: "
                      "same frames posterised to 6 shared luminance bands)",
           fill=(235, 235, 240))

    for col, (path, crop, label) in enumerate((REF, CUR)):
        x = pad + col * (cw + pad)
        im = prep(path, crop, cw, ch)
        sheet.paste(im, (x, top))
        d.text((x, top - 18), label, fill=(210, 210, 220))
        pos, top2 = posterise(im)
        sheet.paste(pos, (x, top + ch + gap))
        a = np.asarray(im, dtype=np.float32)
        L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
        p5, p95 = np.percentile(L, 5), np.percentile(L, 95)
        d.text((x, top + ch + gap - 26),
               f"biggest 2 of 6 bands hold {top2 * 100:.0f}% of the frame    "
               f"L p5 {p5:.0f} -> p95 {p95:.0f}    span {p95 - p5:.0f}",
               fill=(255, 210, 120))

    y = top + ch + gap + ch + 12
    d.text((pad, y), "Both frames touch all six bands, so band COUNT proves nothing. "
                     "The share does: the reference spreads its pixels across the ramp "
                     "(dark eaves, mid stone, lit grass, hot sky),", fill=(200, 200, 210))
    d.text((pad, y + 18), "while the current frame piles them into two neighbouring "
                          "bands. Nothing is dark, nothing is bright except a blown "
                          "sky. That is what reads as 'one orange tone'.",
           fill=(200, 200, 210))
    d.text((pad, y + 44), "Measured by scripts/_flamingo_beauty_axes.py  ·  sheet by "
                          "scripts/_flamingo_beauty_sheet.py", fill=(120, 120, 132))
    Path(out_path).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path)
    print(f"wrote {out_path}  ({W}x{H})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
