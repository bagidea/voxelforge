"""
Monanisa / art lane — hand-authored 16x16 block texture set.
Goal: break the single-hue orange look by giving each material a deliberate
hue/sat/value target (see assets/textures/blocks/PALETTE.md), so a frame
built from these textures has warm wood/roof/sand AND cool stone/glass/
leaves/grass/dock-wood in the same shot, instead of one ambient tint
painting everything the same colour.

Pure PIL, deterministic (seeded per-texture), no external deps beyond Pillow.
Art-lane only — does not touch any Rust code or the build.
"""

import colorsys
import math
import random
from pathlib import Path

from PIL import Image

OUT_DIR = Path(__file__).resolve().parent.parent / "assets" / "textures" / "blocks"
OUT_DIR.mkdir(parents=True, exist_ok=True)

SIZE = 16


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb((h % 360) / 360.0, max(0.0, min(1.0, s)), max(0.0, min(1.0, v)))
    return (round(r * 255), round(g * 255), round(b * 255), 255)


def new_img():
    return Image.new("RGBA", (SIZE, SIZE))


def save(img, name):
    path = OUT_DIR / f"{name}.png"
    img.save(path)
    # average colour for the palette doc / sanity check
    px = list(img.getdata())
    avg = tuple(round(sum(c[i] for c in px) / len(px)) for i in range(3))
    print(f"{name:16s} -> {path.name:24s} avg #{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}")
    return avg


def jitter(rng, base, amt):
    return base + (rng.random() * 2 - 1) * amt


# ---------------------------------------------------------------- oak_planks
def make_oak_planks():
    rng = random.Random("oak_planks")
    img = new_img()
    H, S, V = 30, 0.46, 0.66
    for y in range(SIZE):
        for x in range(SIZE):
            plank = x // 4
            seam = (x % 4 == 3)
            grain = math.sin((y + plank * 3) * 1.3) * 0.04
            v = V + grain + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.03)
            if seam:
                v -= 0.16
                s += 0.04
            # occasional darker grain fleck
            if rng.random() < 0.05:
                v -= 0.12
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# -------------------------------------------------------------- oak_log_side
def make_log_side():
    rng = random.Random("oak_log_side")
    img = new_img()
    H, S, V = 24, 0.42, 0.38
    # irregular vertical bark-crack columns
    cracks = sorted(rng.sample(range(1, SIZE - 1), 4))
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.05)
            dist = min(abs(x - c) for c in cracks)
            if dist == 0:
                v -= 0.16
            elif dist == 1 and rng.random() < 0.5:
                v -= 0.08
            v += math.sin(y * 0.9 + x * 0.2) * 0.02
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# --------------------------------------------------------------- oak_log_top
def make_log_top():
    rng = random.Random("oak_log_top")
    img = new_img()
    cx, cy = 7.5, 7.5
    H = 33
    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy)
            ring = math.sin(d * 1.6) * 0.5 + 0.5  # 0..1 band
            angle = math.atan2(y - cy, x - cx)
            crack = 1.0 if (abs(math.sin(angle * 3)) > 0.985 and d > 1.5) else 0.0
            s = 0.28 + ring * 0.12 + jitter(rng, 0, 0.03)
            v = 0.58 + ring * 0.20 + jitter(rng, 0, 0.03)
            if crack:
                v -= 0.18
                s += 0.05
            if d > 7.3:
                v -= 0.10  # bark rim
                s += 0.10
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# -------------------------------------------------------------- stone_bricks
def make_stone_bricks():
    rng = random.Random("stone_bricks")
    img = new_img()
    H, S, V = 212, 0.10, 0.55
    brick_h = 4
    for y in range(SIZE):
        row = y // brick_h
        offset = 4 if row % 2 else 0
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            mortar = (y % brick_h == 0) or (bx % 8 == 0)
            brick_id = f"stone_bricks-{row}-{(x + offset) // 8}"
            brng = random.Random(brick_id)
            v = V + jitter(brng, 0, 0.06)
            s = S + jitter(brng, 0, 0.03)
            if mortar:
                v -= 0.18
                s -= 0.02
            v += jitter(rng, 0, 0.02)
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 4), s, v))
    return img


# ---------------------------------------------------------------------- sand
def make_sand():
    rng = random.Random("sand")
    img = new_img()
    H, S, V = 42, 0.32, 0.86
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.06)
            if rng.random() < 0.08:
                v -= 0.10  # tiny shadow grains
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), s, v))
    return img


# ----------------------------------------------------------------- grass_top
def make_grass_top():
    rng = random.Random("grass_top")
    img = new_img()
    H, S, V = 102, 0.55, 0.52
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.07)
            s = S + jitter(rng, 0, 0.06)
            if rng.random() < 0.12:
                v -= 0.14  # dark tuft
                s += 0.05
            elif rng.random() < 0.10:
                v += 0.10  # sun-lit blade
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 4), s, v))
    return img


# ---------------------------------------------------------------- grass_side
def make_grass_side():
    rng = random.Random("grass_side")
    img = new_img()
    Hd, Sd, Vd = 26, 0.46, 0.34
    Hg, Sg, Vg = 102, 0.55, 0.52
    # irregular cap line so it reads as grass growing over dirt, not a hard rule
    cap = [3 + (1 if (x in (2, 5, 9, 13)) else 0) for x in range(SIZE)]
    for y in range(SIZE):
        for x in range(SIZE):
            if y < cap[x]:
                v = Vg + jitter(rng, 0, 0.07)
                s = Sg + jitter(rng, 0, 0.05)
                h = Hg + jitter(rng, 0, 4)
            elif y == cap[x]:
                # thin darker fringe between grass and dirt
                v = Vg - 0.14 + jitter(rng, 0, 0.03)
                s = Sg + 0.05
                h = 96
            else:
                v = Vd + jitter(rng, 0, 0.06)
                s = Sd + jitter(rng, 0, 0.05)
                h = Hd + jitter(rng, 0, 3)
                if rng.random() < 0.07:
                    v -= 0.08
            img.putpixel((x, y), hsv(h, s, v))
    return img


# ------------------------------------------------------------------- leaves
def make_leaves():
    rng = random.Random("leaves")
    img = new_img()
    H, S, V = 118, 0.62, 0.42
    for y in range(SIZE):
        for x in range(SIZE):
            cluster = math.sin(x * 0.9 + 1.7) * math.cos(y * 0.8 + 0.4)
            v = V + cluster * 0.10 + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.06)
            if rng.random() < 0.10:
                v -= 0.16  # deep gap between leaf clumps
            elif rng.random() < 0.08:
                v += 0.14  # lit leaf edge
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 5), s, v))
    return img


# ---------------------------------------------------------------- roof_tile
def make_roof_tile():
    rng = random.Random("roof_tile")
    img = new_img()
    H, S, V = 14, 0.60, 0.52
    tile_h = 4
    for y in range(SIZE):
        row = y // tile_h
        offset = 2 if row % 2 else 0
        in_row = y % tile_h
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            edge = in_row == 0 or bx % 4 == 0
            tile_id = f"roof_tile-{row}-{(x + offset) // 4}"
            trng = random.Random(tile_id)
            v = V + jitter(trng, 0, 0.05) - in_row * 0.015
            s = S + jitter(trng, 0, 0.04)
            if edge:
                v -= 0.14
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), s, v))
    return img


# -------------------------------------------------------------------- glass
def make_glass():
    rng = random.Random("glass")
    img = new_img()
    H, S, V = 196, 0.26, 0.88
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.02)
            s = S + jitter(rng, 0, 0.02)
            border = x in (0, SIZE - 1) or y in (0, SIZE - 1)
            streak = abs((x - y) - 3) <= 1
            if border:
                v -= 0.22
                s += 0.06
            elif streak:
                v += 0.09
                s -= 0.08
            img.putpixel((x, y), hsv(H, s, v))
    return img


# --------------------------------------------------------------- clay_plaster
def make_clay_plaster():
    rng = random.Random("clay_plaster")
    img = new_img()
    H, S, V = 204, 0.08, 0.80
    for y in range(SIZE):
        for x in range(SIZE):
            trowel = math.sin((x + y) * 0.5) * 0.02
            v = V + trowel + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.02)
            h = H
            if rng.random() < 0.04:
                h = 222
                s += 0.05
                v -= 0.03
            img.putpixel((x, y), hsv(h, s, v))
    return img


# --------------------------------------------------------------- floorboards
def make_floorboards():
    rng = random.Random("floorboards")
    img = new_img()
    H, S, V = 28, 0.16, 0.58
    for y in range(SIZE):
        board = y // 4
        seam = (y % 4 == 3)
        for x in range(SIZE):
            grain = math.sin((x + board * 5) * 0.6) * 0.03
            v = V + grain + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.03)
            if seam:
                v -= 0.15
            if rng.random() < 0.04:
                v -= 0.10  # weathered grey fleck
                s -= 0.04
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), s, v))
    return img


# ------------------------------------------------------------------- dirt
# Second wave (2026-08-17): the five below fill sim::BlockId names that
# `maps/beach_dusk.json` / `maps/glass_demo.json` actually place and that
# atlas.json's own comment lists as "still on procedural tiles" — plus
# water/metal, which the Director asked for by name even though no
# `BlockId` for either exists yet (see docs/material-palette.md).
def make_dirt():
    rng = random.Random("dirt")
    img = new_img()
    H, S, V = 27, 0.42, 0.36  # matches sim BlockId::DIRT base_color #6b5540
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.06)
            s = S + jitter(rng, 0, 0.05)
            h = H + jitter(rng, 0, 3)
            clump = math.sin(x * 0.7 + 0.3) * math.cos(y * 0.6 + 1.1)
            v += clump * 0.05
            if rng.random() < 0.06:
                v -= 0.14  # dark crumb/root fleck
            elif rng.random() < 0.05:
                v += 0.10  # small pebble catching light
                s -= 0.20
            img.putpixel((x, y), hsv(h, s, v))
    return img


# ------------------------------------------------------------------- brick
def make_brick():
    rng = random.Random("brick")
    img = new_img()
    H, S, V = 20, 0.60, 0.55  # matches sim BlockId::BRICK base_color #965a3c
    brick_h = 4
    for y in range(SIZE):
        row = y // brick_h
        offset = 4 if row % 2 else 0
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            mortar = (y % brick_h == 0) or (bx % 8 == 0)
            brick_id = f"brick-{row}-{(x + offset) // 8}"
            brng = random.Random(brick_id)
            if mortar:
                # mortar reads lighter/greyer than the brick, opposite of
                # stone_bricks — that contrast is what says "clay brick"
                # instead of "masonry" at a glance.
                v = 0.62 + jitter(brng, 0, 0.04)
                s = 0.14 + jitter(brng, 0, 0.03)
                h = 38
            else:
                v = V + jitter(brng, 0, 0.07)
                s = S + jitter(brng, 0, 0.05)
                h = H + jitter(rng, 0, 3)
            img.putpixel((x, y), hsv(h, s, v))
    return img


# -------------------------------------------------------------------- lamp
def make_lamp():
    rng = random.Random("lamp")
    img = new_img()
    cx, cy = 7.5, 7.5
    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy) / 7.5  # 0 centre .. ~1.3 corner
            # bright warm core fading to the ffc476 glow-gradient hex the
            # sim already ships (base_color LAMP) toward the rim.
            h = 40 - d * 12 + jitter(rng, 0, 2)
            s = 0.20 + d * 0.35
            v = 0.97 - d * 0.28 + jitter(rng, 0, 0.02)
            # dark lantern-cage corner posts so it reads as a fixture, not
            # a flat glow tile.
            corner = x in (0, 1, 14, 15) and y in (0, 1, 14, 15)
            if corner:
                h, s, v = 25, 0.35, 0.22
            img.putpixel((x, y), hsv(h, min(s, 0.7), min(v, 1.0)))
    return img


# --------------------------------------------------------------- red_sand
def make_red_sand():
    rng = random.Random("red_sand")
    img = new_img()
    H, S, V = 18, 0.55, 0.78  # matches sim BlockId::RED_SAND base_color #c88246
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.06)
            if rng.random() < 0.08:
                v -= 0.12  # shadow grain, same construction as sand.png
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), s, v))
    return img


# -------------------------------------------------------------------- snow
def make_snow():
    rng = random.Random("snow")
    img = new_img()
    H, S, V = 45, 0.07, 0.94  # matches sim BlockId::SNOW base_color #f0ece0
    for y in range(SIZE):
        for x in range(SIZE):
            dimple = math.sin(x * 0.5 + 0.4) * math.cos(y * 0.45 + 1.0) * 0.02
            v = V + dimple + jitter(rng, 0, 0.02)
            s = S + jitter(rng, 0, 0.02)
            h = H
            if rng.random() < 0.06:
                v = min(1.0, v + 0.05)  # sparkle fleck
                s = 0.03
            elif rng.random() < 0.03:
                h = 205  # rare cool glint, keeps it from reading pure-warm
                s = 0.05
            img.putpixel((x, y), hsv(h, s, v))
    return img


# ------------------------------------------------------------------- water
# No sim BlockId yet (see docs/material-palette.md) — shipped ahead of the
# gameplay hookup since the Director asked for it by name.
def make_water():
    rng = random.Random("water")
    img = new_img()
    H, S, V = 205, 0.55, 0.55
    for y in range(SIZE):
        wave = math.sin(y * 0.9 + math.sin(y * 0.3) * 1.5) * 0.5 + 0.5
        for x in range(SIZE):
            v = V + wave * 0.10 + jitter(rng, 0, 0.03)
            s = S - wave * 0.05 + jitter(rng, 0, 0.03)
            h = H + jitter(rng, 0, 4)
            if rng.random() < 0.05:
                v += 0.20  # sun-glint highlight streak
                s -= 0.15
            img.putpixel((x, y), hsv(h, s, v))
    return img


# ------------------------------------------------------------------- metal
# No sim BlockId yet (see docs/material-palette.md) — shipped ahead of the
# gameplay hookup since the Director asked for it by name.
def make_metal():
    rng = random.Random("metal")
    img = new_img()
    H, S, V = 212, 0.06, 0.62
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.02)
            brushed = math.sin((x - y) * 0.9) * 0.06  # diagonal brushed sheen
            v += brushed
            rivet = (x % 5 == 2) and (y % 5 == 2)
            if rivet:
                v -= 0.18
                s += 0.04
            img.putpixel((x, y), hsv(H, s, v))
    return img


MATERIALS = [
    ("oak_planks", make_oak_planks),
    ("oak_log_side", make_log_side),
    ("oak_log_top", make_log_top),
    ("stone_bricks", make_stone_bricks),
    ("sand", make_sand),
    ("grass_top", make_grass_top),
    ("grass_side", make_grass_side),
    ("leaves", make_leaves),
    ("roof_tile", make_roof_tile),
    ("glass", make_glass),
    ("clay_plaster", make_clay_plaster),
    ("floorboards", make_floorboards),
    ("dirt", make_dirt),
    ("brick", make_brick),
    ("lamp", make_lamp),
    ("red_sand", make_red_sand),
    ("snow", make_snow),
    ("water", make_water),
    ("metal", make_metal),
]


def build_contact_sheet(avgs):
    cols = 5
    rows = math.ceil(len(MATERIALS) / cols)
    scale = 10
    cell = SIZE * scale
    label_h = 22
    pad = 6
    W = cols * (cell + pad) + pad
    Hh = rows * (cell + label_h + pad) + pad
    sheet = Image.new("RGB", (W, Hh), (26, 27, 31))
    from PIL import ImageDraw, ImageFont

    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 13)
    except Exception:
        font = ImageFont.load_default()

    for i, (name, _) in enumerate(MATERIALS):
        r, c = divmod(i, cols)
        x0 = pad + c * (cell + pad)
        y0 = pad + r * (cell + label_h + pad)
        tile = Image.open(OUT_DIR / f"{name}.png").convert("RGB")
        tile_big = tile.resize((cell, cell), Image.NEAREST)
        sheet.paste(tile_big, (x0, y0))
        draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        avg = avgs[name]
        hexcode = f"#{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}"
        draw.text((x0, y0 + cell + 2), f"{name}  {hexcode}", fill=(230, 230, 230), font=font)

    out_path = OUT_DIR / "contact_sheet.png"
    sheet.save(out_path)
    print(f"contact sheet -> {out_path}")


def main():
    avgs = {}
    for name, fn in MATERIALS:
        img = fn()
        assert img.size == (SIZE, SIZE)
        avgs[name] = save(img, name)
    build_contact_sheet(avgs)


if __name__ == "__main__":
    main()
