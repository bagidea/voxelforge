#!/usr/bin/env python3
"""Build the AAA-gap contact sheet: reference targets on top, what the game
actually renders today underneath, at the same pixel height so the gap is a
looking-at problem, not a reading-numbers problem.

usage: python scripts/_flamingo_gap_sheet.py OUT.png  "label=path" "label=path" ...
Rows are laid out two-per-row; each cell is captioned with its label.
"""
import sys
from PIL import Image, ImageDraw, ImageFont

CELL_W, CELL_H, PAD, CAP = 640, 360, 14, 26
BG, FG = (18, 18, 20), (238, 236, 230)

def font(sz):
    for p in ("C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf"):
        try:
            return ImageFont.truetype(p, sz)
        except OSError:
            pass
    return ImageFont.load_default()

out = sys.argv[1]
items = [a.split("=", 1) for a in sys.argv[2:]]
cols = 2
rows = (len(items) + cols - 1) // cols
W = cols * CELL_W + (cols + 1) * PAD
H = rows * (CELL_H + CAP) + (rows + 1) * PAD
sheet = Image.new("RGB", (W, H), BG)
d = ImageDraw.Draw(sheet)
f = font(17)

for i, (label, path) in enumerate(items):
    cx, cy = i % cols, i // cols
    x = PAD + cx * (CELL_W + PAD)
    y = PAD + cy * (CELL_H + CAP + PAD)
    try:
        im = Image.open(path).convert("RGB")
    except Exception as e:                       # missing frame is information too
        d.text((x + 4, y + 4), f"MISSING {path}: {e}", fill=(220, 90, 90), font=f)
        continue
    im.thumbnail((CELL_W, CELL_H), Image.LANCZOS)
    ox = x + (CELL_W - im.width) // 2
    oy = y + (CELL_H - im.height) // 2
    sheet.paste(im, (ox, oy))
    d.rectangle([x, y, x + CELL_W, y + CELL_H], outline=(60, 60, 66))
    d.text((x + 2, y + CELL_H + 5), label, fill=FG, font=f)

sheet.save(out)
print(out, sheet.size)
