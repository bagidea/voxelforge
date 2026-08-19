"""
Monanisa / art lane — boss-facing preview of the full block atlas.

Reads assets/textures/blocks/atlas.json (the same manifest the client
loads at startup) so this preview can never drift from what's actually in
the atlas, and renders every tile at 10x nearest-neighbour with a title
bar into docs/assets/look/atlas-preview.png.

Pure PIL. Art-lane only — does not touch any Rust code or the build.
"""

import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BLOCKS_DIR = ROOT / "assets" / "textures" / "blocks"
OUT_PATH = ROOT / "docs" / "assets" / "look" / "atlas-preview.png"


def avg_hex(img):
    px = list(img.convert("RGB").getdata())
    r = round(sum(c[0] for c in px) / len(px))
    g = round(sum(c[1] for c in px) / len(px))
    b = round(sum(c[2] for c in px) / len(px))
    return f"#{r:02x}{g:02x}{b:02x}"


def main():
    manifest = json.loads((BLOCKS_DIR / "atlas.json").read_text(encoding="utf-8"))
    tiles = manifest["tiles"]
    tile_px = manifest["tile_px"]

    cols = 5
    rows = math.ceil(len(tiles) / cols)
    scale = 10
    cell = tile_px * scale
    label_h = 20
    pad = 8
    title_h = 40

    W = cols * (cell + pad) + pad
    H = rows * (cell + label_h + pad) + pad + title_h
    sheet = Image.new("RGB", (W, H), (22, 23, 27))
    draw = ImageDraw.Draw(sheet)

    try:
        title_font = ImageFont.truetype("arialbd.ttf", 20)
        label_font = ImageFont.truetype("arial.ttf", 13)
    except Exception:
        title_font = ImageFont.load_default()
        label_font = ImageFont.load_default()

    draw.text(
        (pad, 10),
        f"Voxelforge block atlas — {len(tiles)} tiles, {tile_px}x{tile_px}px each",
        fill=(235, 235, 235),
        font=title_font,
    )

    for i, tile in enumerate(tiles):
        r, c = divmod(i, cols)
        x0 = pad + c * (cell + pad)
        y0 = title_h + pad + r * (cell + label_h + pad)
        img = Image.open(BLOCKS_DIR / tile["file"])
        big = img.convert("RGB").resize((cell, cell), Image.NEAREST)
        sheet.paste(big, (x0, y0))
        draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(70, 72, 78), width=1)
        label = f"{tile['name']}  {avg_hex(img)}"
        draw.text((x0, y0 + cell + 2), label, fill=(225, 225, 225), font=label_font)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(OUT_PATH)
    print(f"atlas preview -> {OUT_PATH}  ({len(tiles)} tiles)")


if __name__ == "__main__":
    main()
