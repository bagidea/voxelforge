#!/usr/bin/env python3
"""Contact sheet for the NOHUD beauty pass — labelled with what each tile ACTUALLY is.

The first version of this sheet labelled tile 4 "VISTA / RAKING LIGHT" when the frame
had been shot at the default `Hour::GOLDEN`; the label promised a look the pixels did
not have. So the caption text lives here, next to the shippable flag, and every tile
that is NOT shippable says why on the tile itself.

Usage: python scripts/_poppy_beauty_sheet.py <out.png>
"""
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "_poppy_beauty" / "final"

# (file, title, subtitle, shippable)
TILES = [
    (
        "s1-village-wide.png",
        "1  VILLAGE WIDE",
        "aerial over the campfire quad - shipped GOLDEN look, no env override",
        True,
    ),
    (
        "s2-hero-medium.png",
        "2  HERO MEDIUM",
        "BLOCKED: placeholder capsule over the rig (dodge_parry.rs::dodge_ghost_flash)",
        False,
    ),
    (
        "s3-husk-clash.png",
        "3  HUSK CLASH",
        "BLOCKED: same capsule, AND the blades never meet - needs a pose/timing pass",
        False,
    ),
    (
        "s4-vista-raking.png",
        "4  RUIN VISTA",
        "eye-level across the ruin field - raking sun, VOXELFORGE_LOOK_SUN=6,140,9000",
        True,
    ),
]

COLS = 2
TW = 760
PAD = 14
CAP_H = 46
BG = (16, 16, 18)
OK = (120, 220, 150)
BAD = (245, 130, 110)


def font(size: int) -> ImageFont.FreeTypeFont:
    for name in ("segoeui.ttf", "arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def main(out: str) -> None:
    f_title = font(20)
    f_sub = font(14)

    thumbs = []
    for name, title, sub, ship in TILES:
        im = Image.open(SRC / name).convert("RGB")
        th = TW * im.size[1] // im.size[0]
        thumbs.append((im.resize((TW, th), Image.LANCZOS), title, sub, ship))

    cell_h = max(t.size[1] for t, *_ in thumbs) + CAP_H
    rows = (len(thumbs) + COLS - 1) // COLS
    sheet = Image.new("RGB", (COLS * (TW + PAD) + PAD, rows * (cell_h + PAD) + PAD), BG)
    d = ImageDraw.Draw(sheet)

    for i, (t, title, sub, ship) in enumerate(thumbs):
        x = PAD + (i % COLS) * (TW + PAD)
        y = PAD + (i // COLS) * (cell_h + PAD)
        sheet.paste(t, (x, y))
        col = OK if ship else BAD
        tag = "SHIPPABLE" if ship else "NOT SHIPPABLE"
        d.text((x + 2, y + t.size[1] + 5), title, font=f_title, fill=(238, 238, 242))
        tw = d.textlength(tag, font=f_title)
        d.text((x + TW - tw - 2, y + t.size[1] + 5), tag, font=f_title, fill=col)
        d.text((x + 2, y + t.size[1] + 28), sub, font=f_sub, fill=(178, 178, 186))

    sheet.save(out)
    print(f"{out}  {sheet.size}  {len(thumbs)} tiles")


if __name__ == "__main__":
    main(sys.argv[1])
