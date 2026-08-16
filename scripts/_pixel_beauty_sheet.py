#!/usr/bin/env python3
"""Side-by-side contact sheet for the beauty-shot review.

The rubric's own go/no-go step 1 is "put the rendered frame next to
docs/assets/golden-beauty-shot-ref.png and say whether it is the same look".
This builds that panel so the call is made on pixels rather than on memory.

Every panel is labelled with the file it was actually read from and that file's
md5, because a caption naming one file over pixels from another is the exact
failure this repo has already paid for twice ("caption must match pixels").

Usage: _pixel_beauty_sheet.py <out.png> <label>=<path> [<label>=<path> ...]
"""
import hashlib
import sys

from PIL import Image, ImageDraw

PANEL_H = 520
PAD = 14
BAR = 46
BG = (24, 22, 20)
FG = (238, 232, 222)
DIM = (150, 143, 133)


def md5(p):
    with open(p, "rb") as f:
        return hashlib.md5(f.read()).hexdigest()[:12]


def main():
    if len(sys.argv) < 3:
        sys.exit("usage: _pixel_beauty_sheet.py <out.png> <label>=<path> ...")
    out = sys.argv[1]
    items = []
    for arg in sys.argv[2:]:
        label, _, path = arg.partition("=")
        im = Image.open(path).convert("RGB")
        w = round(im.size[0] * PANEL_H / im.size[1])
        items.append((label, path, md5(path), im.size, im.resize((w, PANEL_H), Image.LANCZOS)))

    total_w = sum(i[4].size[0] for i in items) + PAD * (len(items) + 1)
    sheet = Image.new("RGB", (total_w, PANEL_H + BAR + PAD * 2), BG)
    d = ImageDraw.Draw(sheet)

    x = PAD
    for label, path, h, orig, im in items:
        sheet.paste(im, (x, PAD))
        y = PAD + PANEL_H + 6
        d.text((x, y), f"{label}  {orig[0]}x{orig[1]}", fill=FG)
        d.text((x, y + 14), path, fill=DIM)
        d.text((x, y + 27), f"md5 {h}", fill=DIM)
        x += im.size[0] + PAD

    sheet.save(out)
    print(f"OK {out} {sheet.size[0]}x{sheet.size[1]}")
    for label, path, h, orig, _ in items:
        print(f"   {label:16s} {path}  md5={h}  native={orig[0]}x{orig[1]}")


if __name__ == "__main__":
    main()
