#!/usr/bin/env python3
"""Contact sheet — every frame in a folder on one page, labelled.

Used to pick the showcase angles: reviewing ten 1280x720 grabs one at a time is ten
round trips, and a grid answers "which framing works" in one look.

Usage: python scripts/_poppy_showcase_sheet.py <out.png> <frame.png> ...
"""
import os
import sys

from PIL import Image, ImageDraw

COLS = 3
TW = 480          # thumb width; height follows the source aspect
PAD = 8
LABEL_H = 18
BG = (18, 18, 20)


def main(out: str, paths: list[str]) -> None:
    thumbs = []
    for p in paths:
        im = Image.open(p).convert("RGB")
        th = TW * im.size[1] // im.size[0]
        thumbs.append((os.path.basename(p), im.resize((TW, th), Image.LANCZOS)))

    rows = (len(thumbs) + COLS - 1) // COLS
    cell_h = max(t.size[1] for _, t in thumbs) + LABEL_H
    sheet = Image.new("RGB", (COLS * (TW + PAD) + PAD, rows * (cell_h + PAD) + PAD), BG)
    d = ImageDraw.Draw(sheet)

    for i, (name, t) in enumerate(thumbs):
        x = PAD + (i % COLS) * (TW + PAD)
        y = PAD + (i // COLS) * (cell_h + PAD)
        sheet.paste(t, (x, y))
        d.text((x + 3, y + t.size[1] + 3), name, fill=(230, 230, 235))

    sheet.save(out)
    print(f"{out}  {sheet.size}  {len(thumbs)} frames")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
