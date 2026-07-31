# make_vfx_sheet.py — one contact sheet from the four VFX renders (lane: pixel).
#
# The before plate and the three after beats share a stage, a camera and a light
# rig, so putting them in one image is the honest way to show what this round
# added: anything that differs between cells IS the VFX layer.
#
# Usage:  python scripts/make_vfx_sheet.py
import os
import sys

from PIL import Image, ImageDraw, ImageFont

CELL_W, CELL_H, BAR = 640, 360, 34
ASSETS = os.path.join(os.path.dirname(__file__), "..", "docs", "assets")

CELLS = [
    ("vfx-00-before.png", "BEFORE — layer loaded, nothing fired"),
    ("vfx-01-impact.png", "AFTER (1) impact: sparks + voxel debris + ash + hit flash + trail"),
    ("vfx-02-dissolve.png", "AFTER (2) death: the Unravelling, top-down voxel dissolve"),
    ("vfx-03-campfire.png", "AFTER (3) campfire: coal bed + embers + flicker"),
]


def load_fit(path):
    im = Image.open(path).convert("RGB")
    im.thumbnail((CELL_W, CELL_H), Image.LANCZOS)
    canvas = Image.new("RGB", (CELL_W, CELL_H), (16, 14, 15))
    canvas.paste(im, ((CELL_W - im.width) // 2, (CELL_H - im.height) // 2))
    return canvas


def main():
    try:
        font = ImageFont.truetype("arial.ttf", 17)
    except OSError:
        font = ImageFont.load_default()

    missing = [n for n, _ in CELLS if not os.path.isfile(os.path.join(ASSETS, n))]
    if missing:
        # Fail loudly rather than quietly emitting a sheet with holes in it — a
        # half-filled contact sheet reads as "these are all the shots there are".
        print("FAIL missing renders: " + ", ".join(missing))
        return 1

    cols, rows = 2, 2
    sheet = Image.new("RGB", (CELL_W * cols, (CELL_H + BAR) * rows), (12, 11, 12))
    draw = ImageDraw.Draw(sheet)

    for i, (name, label) in enumerate(CELLS):
        cx, cy = (i % cols) * CELL_W, (i // cols) * (CELL_H + BAR)
        sheet.paste(load_fit(os.path.join(ASSETS, name)), (cx, cy + BAR))
        draw.rectangle([cx, cy, cx + CELL_W, cy + BAR], fill=(26, 24, 26))
        draw.text((cx + 10, cy + 9), label, font=font, fill=(236, 226, 214))

    out = os.path.join(ASSETS, "vfx-before-after-sheet.png")
    sheet.save(out)
    print("SHEET " + os.path.normpath(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
