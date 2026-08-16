#!/usr/bin/env python
"""Poppy — one contact sheet of every camera candidate, so framing is picked by eye.

Also prints the mean luminance of the TOP 15 % of each frame. A castle framing
with real sky in it sits bright and low-variance up there; one that is looking at
the hillside behind the keep does not — which is exactly the mistake this sheet
exists to stop me repeating.
"""
import sys
import os
import glob
from PIL import Image, ImageDraw

BAR = 26
COLS = 3
TH = 420          # thumbnail width
BG = (16, 17, 20)
FG = (232, 234, 238)


def sky_score(im):
    """Mean luminance + spread of the top 15 % of the frame."""
    g = im.convert("L")
    band = g.crop((0, 0, g.width, max(1, int(g.height * 0.15))))
    px = sorted(band.getdata())
    n = len(px)
    return sum(px) / n, px[int(n * 0.05)], px[int(n * 0.95)]


def main(out_dir):
    paths = sorted(p for p in glob.glob(os.path.join(out_dir, "*.png"))
                   if not os.path.basename(p).startswith("CONTACT"))
    if not paths:
        print("no plates")
        return
    thumbs = []
    for p in paths:
        im = Image.open(p).convert("RGB")
        m, lo, hi = sky_score(im)
        print(f"  {os.path.basename(p):<24} top15%  mean {m:6.1f}  p05 {lo:3d}  p95 {hi:3d}")
        th = im.resize((TH, int(im.height * TH / im.width)), Image.LANCZOS)
        thumbs.append((os.path.splitext(os.path.basename(p))[0], th, m))

    w, h = TH, thumbs[0][1].height
    rows = (len(thumbs) + COLS - 1) // COLS
    sheet = Image.new("RGB", (w * COLS, (h + BAR) * rows), BG)
    d = ImageDraw.Draw(sheet)
    for i, (name, th, m) in enumerate(thumbs):
        x, y = (i % COLS) * w, (i // COLS) * (h + BAR)
        sheet.paste(th, (x, y + BAR))
        d.text((x + 8, y + 7), f"{name}   top15% {m:.0f}", fill=FG)
    path = os.path.join(out_dir, "CONTACT.png")
    sheet.save(path)
    print(path)


if __name__ == "__main__":
    main(sys.argv[1])
