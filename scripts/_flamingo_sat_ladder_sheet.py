#!/usr/bin/env python3
"""Flamingo — the saturation ladder as ONE picture (2026-08-05).

The P0 chromatic axes and the eye disagree on this scene, and a table of numbers
cannot show that. This lays the sweep rungs side by side with their own measured
axes printed under them, so "the row that passes is the row that looks wrong" is
something a reviewer sees rather than something they take on trust.

Numbers are measured at render time by scripts/grade_axes.py (the single source
of truth for the axes) — none is typed in.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import grade_axes  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

SW = "_fl_grade_sweep"
# stem, caption
RUNGS = [
    ("boot-r00-ctrl", "sat 1.05  shipped lights\nSHIPPED DEFAULT"),
    ("boot-r32-s135G", "sat 1.35  G-lift lights\nSHIPPED NOW (this change)"),
    ("boot-r20-s145G", "sat 1.45  G-lift lights"),
    ("boot-r21-s175G", "sat 1.75  G-lift lights\nfirst rung the axes accept"),
    ("boot-r05-s190L", "sat 1.90  B-drained lights\n(2026-08-01 prescription)"),
    ("boot-r06-s230L", "sat 2.30  B-drained lights\nALL AXES PASS"),
]
COLS, TW = 3, 620
PAD, CAPH = 14, 96


def font(sz, bold=False):
    for name in (("arialbd.ttf", "arial.ttf") if bold else ("arial.ttf",)):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            pass
    return ImageFont.load_default()


def main():
    tiles = []
    for stem, cap in RUNGS:
        p = f"{SW}/{stem}-nohud2.png"
        if not os.path.exists(p):
            print(f"missing {p}")
            continue
        m = grade_axes.measure(p)
        im = Image.open(p).convert("RGB")
        th = round(TW * im.height / im.width)
        tiles.append((im.resize((TW, th), Image.LANCZOS), cap, m))
    if not tiles:
        sys.exit("no tiles")

    th = tiles[0][0].height
    rows = (len(tiles) + COLS - 1) // COLS
    W = COLS * TW + (COLS + 1) * PAD
    H = rows * (th + CAPH) + (rows + 1) * PAD + 52
    sheet = Image.new("RGB", (W, H), (22, 22, 26))
    d = ImageDraw.Draw(sheet)
    d.text((PAD, 14), "Voxelforge — POST_SATURATION ladder, --play boot frame, one binary "
                      "(2026-08-05, Flamingo).  Axis targets: warmth >=110 · blue <=10 · sat >=90",
           font=font(19, True), fill=(235, 235, 240))

    for i, (im, cap, m) in enumerate(tiles):
        r, c = divmod(i, COLS)
        x = PAD + c * (TW + PAD)
        y = 52 + PAD + r * (th + CAPH + PAD)
        sheet.paste(im, (x, y))
        d.rectangle([x, y, x + TW - 1, y + th - 1], outline=(70, 70, 78))
        ty = y + th + 6
        for line in cap.split("\n"):
            d.text((x + 4, ty), line, font=font(17, True), fill=(240, 240, 245))
            ty += 21
        ok = (m["warmth"] >= 110, m["blue"] <= 10, m["sat"] >= 90)
        d.text((x + 4, ty + 2),
               f"warmth {m['warmth']:6.1f} {'PASS' if ok[0] else 'FAIL'}   "
               f"blue {m['blue']:5.1f} {'PASS' if ok[1] else 'FAIL'}   "
               f"sat {m['sat']:5.1f} {'PASS' if ok[2] else 'FAIL'}",
               font=font(16), fill=(120, 230, 140) if all(ok) else (235, 175, 110))

    out = "docs/assets/look-2026-08-05/sat-ladder.png"
    os.makedirs(os.path.dirname(out), exist_ok=True)
    sheet.save(out)
    print(out, sheet.size)


if __name__ == "__main__":
    main()
