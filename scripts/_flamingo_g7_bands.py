#!/usr/bin/env python3
"""Eye-check panel for the stone-collateral table in the G7 report.

The scar this exists for: per-axis numbers can all move the "right" way while
the frame is visibly wrong, so every number in the report gets a panel a human
can look at. Left = haze off (G6 frame), middle = shipped haze, right = the
stone mask coloured by |delta| so the depth gradient is visible rather than
asserted. The two horizontal rules are the tercile cuts the numbers use.

Usage: _flamingo_g7_bands.py <hazeoff.png> <ship.png> <out.png>
"""
import colorsys
import sys

from PIL import Image, ImageDraw


def hsv(p):
    h, s, v = colorsys.rgb_to_hsv(p[0] / 255, p[1] / 255, p[2] / 255)
    return h * 360, s * 100, v * 100


off_im = Image.open(sys.argv[1]).convert("RGB")
on_im = Image.open(sys.argv[2]).convert("RGB")
if off_im.size != on_im.size:
    raise SystemExit("! size mismatch")
W, H = off_im.size
op, np_ = off_im.load(), on_im.load()

stone = []
for y in range(H):
    for x in range(W):
        h, s, v = hsv(op[x, y])
        if not (180 <= h <= 260 and s > 12 and v > 40):
            if (h < 40 or h > 330) and s > 12 and v > 25:
                stone.append((x, y))

ys = sorted(y for _, y in stone)
c1, c2 = ys[len(ys) // 3], ys[2 * len(ys) // 3]

# delta panel: black frame, stone pixels lit by |delta| (max channel), hot = moved most
d_im = Image.new("RGB", (W, H), (10, 10, 14))
dp = d_im.load()
for x, y in stone:
    d = max(abs(op[x, y][i] - np_[x, y][i]) for i in range(3))
    t = min(1.0, d / 60.0)
    dp[x, y] = (int(255 * t), int(90 * t), int(40 + 160 * (1 - t)))

sheet = Image.new("RGB", (W * 3, H + 26), (16, 16, 20))
for i, im in enumerate((off_im, on_im, d_im)):
    sheet.paste(im, (W * i, 26))

dr = ImageDraw.Draw(sheet)
for i, label in enumerate(("hazeoff (G6 frame)", "ship (haze default)",
                           "stone mask, |delta| -- hot = moved most")):
    dr.text((W * i + 8, 8), label, fill=(230, 230, 235))
for cy, name in ((c1, "far / mid cut"), (c2, "mid / near cut")):
    dr.line([(0, 26 + cy), (W * 3, 26 + cy)], fill=(80, 200, 255), width=1)
    dr.text((6, 26 + cy + 3), name, fill=(80, 200, 255))

sheet.save(sys.argv[3])
print(f"wrote {sys.argv[3]}  ({sheet.size[0]}x{sheet.size[1]})  cuts y={c1}/{c2}")
