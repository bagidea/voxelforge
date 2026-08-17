"""
Monanisa / art lane — player-character reference sheet.

Renders the 14-box LOD body from docs/player-character-art.md as flat
orthographic front/side/back silhouettes (screen-space rectangles, painter's-
algorithm back-to-front — this is a design reference, not a 3D render), plus
a swatch legend of the six palette colours and the five shipped texture
tiles. Output: docs/assets/look/atlas-preview.png's sibling for the player,
docs/assets/look/player-ref.png.

Pure PIL. Art-lane only — does not touch any Rust code or the build.
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
CHAR_DIR = ROOT / "assets" / "textures" / "character"
OUT_PATH = ROOT / "docs" / "assets" / "look" / "player-ref.png"
OUT_PATH.parent.mkdir(parents=True, exist_ok=True)

# Palette — must match docs/player-character-art.md exactly.
SKIN = (0xD9, 0xB0, 0x8C)
HAIR = (0x2A, 0x1B, 0x12)
TUNIC = (0x2E, 0x6E, 0x78)
LEGWEAR = (0x4A, 0x32, 0x20)
BOOTS = (0x2A, 0x1E, 0x16)
TRIM = (0xE8, 0xA2, 0x3C)

# The 14-box body, in metres, local space (feet at y=0, crown at y=1.8).
# (name, x_center, x_half, y0, y1, z_center, z_half, color)
# x/y here are body-local; z_half is unused for the 2D silhouette but kept
# for the side-view depth pass.
BOXES = [
    ("boot_l", -0.11, 0.10, 0.00, 0.22, 0.00, 0.12, BOOTS),
    ("boot_r",  0.11, 0.10, 0.00, 0.22, 0.00, 0.12, BOOTS),
    ("leg_l",  -0.11, 0.095, 0.22, 0.86, 0.00, 0.10, LEGWEAR),
    ("leg_r",   0.11, 0.095, 0.22, 0.86, 0.00, 0.10, LEGWEAR),
    ("belt",    0.00, 0.20, 0.83, 0.88, 0.00, 0.14, TRIM),
    ("waist",   0.00, 0.16, 0.86, 1.06, 0.00, 0.13, TUNIC),
    ("chest",   0.00, 0.27, 1.06, 1.46, 0.00, 0.16, TUNIC),
    ("cloak",   0.00, 0.20, 1.00, 1.42, 0.21, 0.05, TRIM),
    ("arm_l",  -0.335, 0.055, 0.60, 1.40, 0.00, 0.06, TUNIC),
    ("arm_r",   0.335, 0.055, 0.60, 1.40, 0.00, 0.06, TUNIC),
    ("hand_l", -0.335, 0.06, 0.52, 0.60, 0.00, 0.06, SKIN),
    ("hand_r",  0.335, 0.06, 0.52, 0.60, 0.00, 0.06, SKIN),
    ("head",    0.00, 0.155, 1.46, 1.74, 0.00, 0.155, SKIN),
    ("hair",    0.00, 0.17, 1.72, 1.80, 0.00, 0.17, HAIR),
]

SCALE = 260  # px per metre
MARGIN = 40
BODY_H = int(1.9 * SCALE)
BODY_W = int(0.9 * SCALE)
BG = (18, 19, 22)
GROUND = (40, 42, 47)


def draw_front_or_back(draw, ox, oy, flip_x):
    """flip_x mirrors L/R (used for the back view so the silhouette isn't
    just a copy — cloak reads at the +Z/back, so the back view shows it
    covering the torso instead of hidden behind it)."""
    for name, xc, xh, y0, y1, _zc, _zh, color in BOXES:
        if name == "cloak" and not flip_x:
            continue  # front view: cloak is behind the body, not visible
        x = xc * (-1 if flip_x else 1)
        px0 = ox + int((x - xh) * SCALE) + BODY_W // 2
        px1 = ox + int((x + xh) * SCALE) + BODY_W // 2
        py0 = oy + BODY_H - int(y1 * SCALE)
        py1 = oy + BODY_H - int(y0 * SCALE)
        draw.rectangle([px0, py0, px1, py1], fill=color, outline=(0, 0, 0, 40))


def draw_side(draw, ox, oy):
    # Side view: use z-depth as the horizontal axis instead of x.
    for name, _xc, _xh, y0, y1, zc, zh, color in BOXES:
        pz0 = ox + int((zc - zh) * SCALE) + BODY_W // 2
        pz1 = ox + int((zc + zh) * SCALE) + BODY_W // 2
        py0 = oy + BODY_H - int(y1 * SCALE)
        py1 = oy + BODY_H - int(y0 * SCALE)
        draw.rectangle([pz0, py0, pz1, py1], fill=color, outline=(0, 0, 0, 40))


def label_view(draw, ox, oy, title, font):
    draw.rectangle([ox, oy + BODY_H, ox + BODY_W, oy + BODY_H + 6], fill=GROUND)
    tw = draw.textlength(title, font=font)
    draw.text((ox + BODY_W / 2 - tw / 2, oy + BODY_H + 12), title, fill=(220, 220, 220), font=font)


def main():
    cols = 3
    panel_w = BODY_W + 60
    panel_h = BODY_H + 60
    legend_rows = 6
    legend_row_h = 46
    swatch_h = 40 + legend_rows * legend_row_h  # header + 6 palette rows
    W = cols * panel_w + MARGIN * 2
    H = 60 + panel_h + 40 + swatch_h + MARGIN

    sheet = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(sheet)

    try:
        font_title = ImageFont.truetype("arial.ttf", 22)
        font = ImageFont.truetype("arial.ttf", 15)
        font_small = ImageFont.truetype("arial.ttf", 12)
    except Exception:
        font_title = font = font_small = ImageFont.load_default()

    draw.text((MARGIN, 14), "Voxelforge player character — LOD body reference (14 boxes)",
              fill=(240, 240, 240), font=font_title)

    top = 60
    views = [("FRONT", False, "front"), ("SIDE", False, "side"), ("BACK", True, "back")]
    for i, (title, flip, kind) in enumerate(views):
        ox = MARGIN + i * panel_w
        oy = top
        draw.rectangle([ox, oy, ox + BODY_W, oy + BODY_H], fill=(28, 29, 33))
        if kind == "side":
            draw_side(draw, ox, oy)
        else:
            draw_front_or_back(draw, ox, oy, flip)
        label_view(draw, ox, oy, title, font)

    # ---- swatch legend --------------------------------------------------
    legend_y = top + panel_h + 40
    draw.text((MARGIN, legend_y), "Palette", fill=(230, 230, 230), font=font)
    swatches = [
        ("Skin", SKIN, "reused: equipment.rs Surf::Skin"),
        ("Hair", HAIR, "reused: equipment.rs Surf::Hair"),
        ("Tunic", TUNIC, "new — cool anchor, H188"),
        ("Legwear", LEGWEAR, "reused: Surf::LeatherStrap"),
        ("Boots", BOOTS, "new — darkest value"),
        ("Trim", TRIM, "new — high-sat accent"),
    ]
    sw = 26
    sx = MARGIN
    sy = legend_y + 26
    for name, color, note in swatches:
        draw.rectangle([sx, sy, sx + sw, sy + sw], fill=color, outline=(80, 80, 80))
        hexcode = f"#{color[0]:02x}{color[1]:02x}{color[2]:02x}"
        draw.text((sx + sw + 8, sy), f"{name}  {hexcode}", fill=(220, 220, 220), font=font_small)
        draw.text((sx + sw + 8, sy + 15), note, fill=(150, 150, 150), font=font_small)
        sy += sw + 20

    # ---- texture tile strip ----------------------------------------------
    tiles_x = MARGIN + 340
    draw.text((tiles_x, legend_y), "Shipped textures (assets/textures/character/, 16x16)",
              fill=(230, 230, 230), font=font)
    tile_scale = 6
    tile_gap = 14
    tx = tiles_x
    ty = legend_y + 26
    for name in ("skin", "shirt", "pants", "boots", "cloak"):
        tile = Image.open(CHAR_DIR / f"{name}.png").convert("RGB")
        big = tile.resize((16 * tile_scale, 16 * tile_scale), Image.NEAREST)
        sheet.paste(big, (tx, ty))
        draw.rectangle([tx, ty, tx + 16 * tile_scale, ty + 16 * tile_scale], outline=(80, 80, 80))
        draw.text((tx, ty + 16 * tile_scale + 4), name, fill=(200, 200, 200), font=font_small)
        tx += 16 * tile_scale + tile_gap

    sheet.save(OUT_PATH)
    print(f"player reference sheet -> {OUT_PATH}")


if __name__ == "__main__":
    main()
