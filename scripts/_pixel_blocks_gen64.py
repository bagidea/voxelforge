"""
Monanisa / art lane — 64x64 PBR upgrade of the hand-authored block set.

Texture lane was the single biggest gap vs a realistic pack: all 19 blocks
were 16x16 albedo-only. This script re-authors every material at 64x64 and
ships a full PBR triplet per block:

  <name>.png    albedo   — same HSV target/hue rationale as the 16px set
                           (see PALETTE.md), redrawn with real per-material
                           structure at 4x the resolution: brick/stone
                           mortar courses, individual wood grain fibers,
                           per-grain sand/dirt speckle, bark cracks, brick
                           cage bars, etc. Not upscaled noise.
  <name>_n.png  normal   — tangent-space normal map derived from a height
                           field that is authored IN THE SAME per-pixel pass
                           as the albedo (grooves/mortar/seams recess the
                           height, grain ridges/highlights raise it), so the
                           bump geometry always matches what the albedo
                           actually draws.
  <name>_r.png  roughness — grayscale (white = rough, black = mirror-smooth).
                           Per-material base (glass/water near-mirror, metal
                           semi-glossy, dirt/sand/cloth near-fully-rough)
                           modulated by the same height delta: recessed
                           grooves read rougher (grime), raised/lit texels
                           read smoother — same physical logic in every
                           material instead of bespoke per-map noise.

Height -> (normal, roughness) is derived generically via `emit()`: every
make_* function computes its final albedo value `v` and the *local* baseline
`v_ref` for whatever region of the tile it's in (e.g. brick vs mortar, grass
vs dirt), and `emit` turns `dv = v - v_ref` into a correlated height/rough
delta. This keeps 19 bespoke pattern functions from needing 19 bespoke
normal/rough authoring passes, while still tying the bump/gloss detail to
real drawn structure instead of decorrelated noise.

Pure PIL, deterministic (seeded per-texture) — re-running reproduces
byte-identical output. Art-lane only, no Rust/build changes: the manifest
(atlas.json) gains `normal`/`roughness` keys the current loader simply
ignores (no `deny_unknown_fields` on its TileEntry) until the render lane
wires them up.
"""

import colorsys
import json
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets" / "textures" / "blocks"
BACKUP_DIR = ROOT / "assets" / "textures" / "blocks_16px_backup"
OUT_DIR.mkdir(parents=True, exist_ok=True)

SIZE = 64
SCALE = SIZE // 16  # 4x the original 16px set

HEIGHT_GAIN = 1.8
ROUGH_GAIN = 0.55


def clamp01(v):
    return max(0.0, min(1.0, v))


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb((h % 360) / 360.0, clamp01(s), clamp01(v))
    return (round(r * 255), round(g * 255), round(b * 255), 255)


def jitter(rng, base, amt):
    return base + (rng.random() * 2 - 1) * amt


def new_state():
    albedo = Image.new("RGBA", (SIZE, SIZE))
    height = [[0.5] * SIZE for _ in range(SIZE)]
    rough = [[0.5] * SIZE for _ in range(SIZE)]
    return albedo, height, rough


def emit(albedo, height, rough, x, y, h, s, v, v_ref, base_rough, rough_override=None):
    """Write one texel of albedo + the height/roughness it implies."""
    dv = v - v_ref
    albedo.putpixel((x, y), hsv(h, s, v))
    height[y][x] = clamp01(0.5 + dv * HEIGHT_GAIN)
    rough[y][x] = rough_override if rough_override is not None else clamp01(base_rough - dv * ROUGH_GAIN)


def normal_from_height(height, strength=2.4):
    img = Image.new("RGBA", (SIZE, SIZE))
    for y in range(SIZE):
        for x in range(SIZE):
            hl = height[y][(x - 1) % SIZE]
            hr = height[y][(x + 1) % SIZE]
            hu = height[(y - 1) % SIZE][x]
            hd = height[(y + 1) % SIZE][x]
            dx = (hr - hl) * strength
            dy = (hd - hu) * strength
            nx, ny, nz = -dx, -dy, 1.0
            length = math.sqrt(nx * nx + ny * ny + nz * nz)
            nx, ny, nz = nx / length, ny / length, nz / length
            r = round((nx * 0.5 + 0.5) * 255)
            g = round((ny * 0.5 + 0.5) * 255)
            b = round((nz * 0.5 + 0.5) * 255)
            img.putpixel((x, y), (r, g, b, 255))
    return img


def roughness_img(rough):
    img = Image.new("RGBA", (SIZE, SIZE))
    for y in range(SIZE):
        for x in range(SIZE):
            v = round(clamp01(rough[y][x]) * 255)
            img.putpixel((x, y), (v, v, v, 255))
    return img


def save(img, name, suffix=""):
    path = OUT_DIR / f"{name}{suffix}.png"
    img.save(path)
    return path


def avg_hex(img):
    px = list(img.convert("RGB").getdata())
    avg = tuple(round(sum(c[i] for c in px) / len(px)) for i in range(3))
    return f"#{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}", avg


# ---------------------------------------------------------------- oak_planks
def make_oak_planks():
    albedo, height, rough = new_state()
    rng = random.Random("oak_planks")
    H, S, V = 30, 0.46, 0.66
    BR = 0.72
    plank_w = 16  # 4 planks across 64px, same board count as the 16px set
    for y in range(SIZE):
        for x in range(SIZE):
            plank = x // plank_w
            within = x % plank_w
            seam = within >= plank_w - 2
            # coarse board-to-board tone variance
            board_rng = random.Random(f"oak_planks-board-{plank}")
            board_tone = jitter(board_rng, 0, 0.03)
            # long grain fiber striations running along the board (y axis)
            fiber = math.sin((y * 0.9 + plank * 5.1)) * 0.025 + math.sin(y * 3.7 + x * 0.4) * 0.012
            v_ref = V + board_tone
            v = v_ref + fiber + jitter(rng, 0, 0.02)
            s = S + jitter(rng, 0, 0.03)
            rr = BR
            if seam:
                v -= 0.18
                s += 0.05
                rr += 0.06
            if rng.random() < 0.03:
                v -= 0.10  # knot/grain fleck
                rr += 0.04
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 2), s, v, v_ref, rr)
    return albedo, height, rough


# -------------------------------------------------------------- oak_log_side
def make_log_side():
    albedo, height, rough = new_state()
    rng = random.Random("oak_log_side")
    H, S, V = 24, 0.42, 0.38
    BR = 0.88
    cracks = sorted(rng.sample(range(2, SIZE - 2), 12))
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            v = V + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.05)
            dist = min(abs(x - c) for c in cracks)
            rr = BR
            if dist == 0:
                v -= 0.18
                rr += 0.08
            elif dist <= 1 and rng.random() < 0.5:
                v -= 0.09
                rr += 0.04
            v += math.sin(y * 0.35 + x * 0.08) * 0.025  # bark plate waviness
            v += math.sin(y * 1.6 + x * 0.5) * 0.015  # fine bark fiber
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 2), s, v, v_ref, rr)
    return albedo, height, rough


# --------------------------------------------------------------- oak_log_top
def make_log_top():
    albedo, height, rough = new_state()
    rng = random.Random("oak_log_top")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
    H = 33
    BR = 0.60
    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy)
            ring = math.sin(d * 0.42) * 0.5 + 0.5
            angle = math.atan2(y - cy, x - cx)
            crack = 1.0 if (abs(math.sin(angle * 7)) > 0.985 and d > 6) else 0.0
            s = 0.28 + ring * 0.12 + jitter(rng, 0, 0.03)
            v_ref = 0.58 + ring * 0.20
            v = v_ref + jitter(rng, 0, 0.03)
            rr = BR - ring * 0.08
            if crack:
                v -= 0.20
                s += 0.05
                rr += 0.10
            if d > 29:
                v -= 0.10  # bark rim
                s += 0.10
                rr += 0.15
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 2), s, v, v_ref, rr)
    return albedo, height, rough


# -------------------------------------------------------------- stone_bricks
def make_stone_bricks():
    albedo, height, rough = new_state()
    rng = random.Random("stone_bricks")
    H, S, V = 212, 0.10, 0.55
    brick_h = 16
    for y in range(SIZE):
        row = y // brick_h
        offset = 16 if row % 2 else 0
        in_row = y % brick_h
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            mortar = in_row < 2 or bx % 32 < 2
            brick_id = f"stone_bricks-{row}-{(x + offset) // 32}"
            brng = random.Random(brick_id)
            v_ref = V + jitter(brng, 0, 0.06)
            # fine mineral speckle inside each stone
            speck = jitter(rng, 0, 0.02)
            v = v_ref + speck
            s = S + jitter(brng, 0, 0.03) + jitter(rng, 0, 0.01)
            rr = 0.80
            if mortar:
                v -= 0.20
                s -= 0.02
                rr = 0.90
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 4), s, v, v_ref, rr)
    return albedo, height, rough


# ---------------------------------------------------------------------- sand
def make_sand():
    albedo, height, rough = new_state()
    rng = random.Random("sand")
    H, S, V = 42, 0.32, 0.86
    BR = 0.93
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            v = V + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.05)
            rr = BR
            g = rng.random()
            if g < 0.10:
                v -= 0.10  # individual grain shadow
                rr += 0.03
            elif g < 0.14:
                v += 0.08  # sunlit grain crest
                rr -= 0.05
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 3), s, v, v_ref, rr)
    return albedo, height, rough


# ----------------------------------------------------------------- grass_top
def make_grass_top():
    albedo, height, rough = new_state()
    rng = random.Random("grass_top")
    H, S, V = 102, 0.55, 0.52
    BR = 0.86
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V + math.sin(x * 0.5 + y * 0.3) * 0.02
            v = v_ref + jitter(rng, 0, 0.06)
            s = S + jitter(rng, 0, 0.06)
            rr = BR
            g = rng.random()
            if g < 0.16:
                v -= 0.16  # dark tuft / blade shadow
                s += 0.05
                rr += 0.05
            elif g < 0.26:
                v += 0.11  # sunlit blade tip
                rr -= 0.06
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 4), s, v, v_ref, rr)
    return albedo, height, rough


# ---------------------------------------------------------------- grass_side
def make_grass_side():
    albedo, height, rough = new_state()
    rng = random.Random("grass_side")
    Hd, Sd, Vd = 26, 0.46, 0.34
    Hg, Sg, Vg = 102, 0.55, 0.52
    # irregular grass/dirt cap line, widened to the 64px canvas
    cap = [4 * SCALE + (2 if (x // SCALE) in (2, 5, 9, 13) else 0) + int(2 * math.sin(x * 0.4)) for x in range(SIZE)]
    for y in range(SIZE):
        for x in range(SIZE):
            if y < cap[x]:
                v_ref = Vg
                v = v_ref + jitter(rng, 0, 0.06) + (math.sin(x * 1.1 + y * 0.6) * 0.02)
                s = Sg + jitter(rng, 0, 0.05)
                h = Hg + jitter(rng, 0, 4)
                rr = 0.86
                if rng.random() < 0.10:
                    v -= 0.14
                    rr += 0.05
            elif y < cap[x] + 2:
                v_ref = Vg - 0.14
                v = v_ref + jitter(rng, 0, 0.03)
                s = Sg + 0.05
                h = 96
                rr = 0.88
            else:
                v_ref = Vd
                v = v_ref + jitter(rng, 0, 0.05)
                s = Sd + jitter(rng, 0, 0.05)
                h = Hd + jitter(rng, 0, 3)
                rr = 0.92
                if rng.random() < 0.08:
                    v -= 0.09
                    rr += 0.03
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# ------------------------------------------------------------------- leaves
def make_leaves():
    albedo, height, rough = new_state()
    rng = random.Random("leaves")
    H, S, V = 118, 0.62, 0.42
    BR = 0.80
    for y in range(SIZE):
        for x in range(SIZE):
            cluster = math.sin(x * 0.35 + 1.7) * math.cos(y * 0.32 + 0.4)
            leaf_edge = math.sin(x * 1.3 + y * 0.9) * math.cos(x * 0.6 - y * 1.1)
            v_ref = V + cluster * 0.10
            v = v_ref + leaf_edge * 0.03 + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.06)
            rr = BR
            g = rng.random()
            if g < 0.11:
                v -= 0.18  # deep gap between clumps
                rr += 0.06
            elif g < 0.19:
                v += 0.15  # lit leaf edge
                rr -= 0.08
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 5), s, v, v_ref, rr)
    return albedo, height, rough


# ---------------------------------------------------------------- roof_tile
def make_roof_tile():
    albedo, height, rough = new_state()
    rng = random.Random("roof_tile")
    H, S, V = 14, 0.60, 0.52
    tile_h = 16
    for y in range(SIZE):
        row = y // tile_h
        offset = 8 if row % 2 else 0
        in_row = y % tile_h
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            edge = in_row < 2 or bx % 16 < 2
            tile_id = f"roof_tile-{row}-{(x + offset) // 16}"
            trng = random.Random(tile_id)
            v_ref = V + jitter(trng, 0, 0.05)
            # curved-tile shading: brighter at each tile's crown, darker at its edges
            crown = math.sin((in_row / tile_h) * math.pi)
            v = v_ref + crown * 0.05 - in_row * 0.006
            s = S + jitter(trng, 0, 0.04)
            rr = 0.66 - crown * 0.05
            if edge:
                v -= 0.16
                rr += 0.10
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 3), s, v, v_ref, rr)
    return albedo, height, rough


# -------------------------------------------------------------------- glass
def make_glass():
    albedo, height, rough = new_state()
    rng = random.Random("glass")
    H, S, V = 196, 0.26, 0.88
    BR = 0.06
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            v = v_ref + jitter(rng, 0, 0.02)
            s = S + jitter(rng, 0, 0.02)
            border = x < 3 or x >= SIZE - 3 or y < 3 or y >= SIZE - 3
            mullion = (x % 32 < 2) or (y % 32 < 2)
            streak = abs((x - y) - 12) <= 4
            rr = BR
            if border or mullion:
                v -= 0.24
                s += 0.06
                rr = 0.35  # frame reads matte vs the pane
            elif streak:
                v += 0.10
                s -= 0.08
                rr = 0.02  # specular streak, near-mirror
            emit(albedo, height, rough, x, y, H, s, v, v_ref, rr, rough_override=rr)
    return albedo, height, rough


# --------------------------------------------------------------- clay_plaster
def make_clay_plaster():
    albedo, height, rough = new_state()
    rng = random.Random("clay_plaster")
    H, S, V = 204, 0.08, 0.80
    BR = 0.78
    for y in range(SIZE):
        for x in range(SIZE):
            trowel = math.sin((x + y) * 0.18) * 0.025 + math.sin(x * 0.6 - y * 0.4) * 0.012
            v_ref = V + trowel
            v = v_ref + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.02)
            h = H
            rr = BR
            if rng.random() < 0.03:
                h = 222
                s += 0.05
                v -= 0.04
                rr += 0.05
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# --------------------------------------------------------------- floorboards
def make_floorboards():
    albedo, height, rough = new_state()
    rng = random.Random("floorboards")
    H, S, V = 28, 0.16, 0.58
    board_h = 16
    for y in range(SIZE):
        board = y // board_h
        within = y % board_h
        seam = within >= board_h - 2
        for x in range(SIZE):
            board_rng = random.Random(f"floorboards-board-{board}")
            board_tone = jitter(board_rng, 0, 0.03)
            v_ref = V + board_tone
            grain = math.sin((x + board * 5) * 0.15) * 0.03 + math.sin(x * 1.1 + board) * 0.012
            v = v_ref + grain + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.03)
            rr = 0.66
            if seam:
                v -= 0.17
                rr += 0.10
            if rng.random() < 0.03:
                v -= 0.11  # weathered grey fleck
                s -= 0.04
                rr += 0.05
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 3), s, v, v_ref, rr)
    return albedo, height, rough


# ------------------------------------------------------------------- dirt
def make_dirt():
    albedo, height, rough = new_state()
    rng = random.Random("dirt")
    H, S, V = 27, 0.42, 0.36  # matches sim BlockId::DIRT #6b5540
    BR = 0.93
    for y in range(SIZE):
        for x in range(SIZE):
            clump = math.sin(x * 0.18 + 0.3) * math.cos(y * 0.15 + 1.1)
            v_ref = V + clump * 0.05
            v = v_ref + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.05)
            h = H + jitter(rng, 0, 3)
            rr = BR
            g = rng.random()
            if g < 0.05:
                v -= 0.16  # dark crumb/root fleck
                rr += 0.03
            elif g < 0.09:
                v += 0.11  # small pebble catching light
                s -= 0.20
                rr -= 0.12
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# ------------------------------------------------------------------- brick
def make_brick():
    albedo, height, rough = new_state()
    rng = random.Random("brick")
    H, S, V = 20, 0.60, 0.55  # matches sim BlockId::BRICK #965a3c
    brick_h = 16
    for y in range(SIZE):
        row = y // brick_h
        offset = 16 if row % 2 else 0
        in_row = y % brick_h
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            mortar = in_row < 2 or bx % 32 < 2
            brick_id = f"brick-{row}-{(x + offset) // 32}"
            brng = random.Random(brick_id)
            if mortar:
                v_ref = 0.62
                v = v_ref + jitter(brng, 0, 0.04)
                s = 0.14 + jitter(brng, 0, 0.03)
                h = 38
                rr = 0.88
            else:
                v_ref = V + jitter(brng, 0, 0.07)
                fire_speck = math.sin(x * 0.9 + y * 0.7 + row) * 0.02
                v = v_ref + fire_speck + jitter(rng, 0, 0.02)
                s = S + jitter(brng, 0, 0.05)
                h = H + jitter(rng, 0, 3)
                rr = 0.76
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# -------------------------------------------------------------------- lamp
def make_lamp():
    albedo, height, rough = new_state()
    rng = random.Random("lamp")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy) / (SIZE / 2 * 0.98)
            h = 40 - d * 12 + jitter(rng, 0, 2)
            s = 0.20 + d * 0.35
            v_ref = 0.97 - d * 0.28
            v = v_ref + jitter(rng, 0, 0.02)
            rr = 0.35 + d * 0.15
            frame = (x % 16 < 2) or (y % 16 < 2)
            corner = x < 6 or x >= SIZE - 6 or y < 6 or y >= SIZE - 6
            if corner:
                h, s, v_ref = 25, 0.35, 0.22
                v = v_ref + jitter(rng, 0, 0.02)
                rr = 0.62
            elif frame:
                v -= 0.08
                rr += 0.08
            emit(albedo, height, rough, x, y, h, min(s, 0.7), min(v, 1.0), v_ref, rr)
    return albedo, height, rough


# --------------------------------------------------------------- red_sand
def make_red_sand():
    albedo, height, rough = new_state()
    rng = random.Random("red_sand")
    H, S, V = 18, 0.55, 0.78  # matches sim BlockId::RED_SAND #c88246
    BR = 0.92
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            v = v_ref + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.06)
            rr = BR
            g = rng.random()
            if g < 0.10:
                v -= 0.09
                rr += 0.03
            elif g < 0.13:
                v += 0.07
                rr -= 0.05
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 3), s, v, v_ref, rr)
    return albedo, height, rough


# -------------------------------------------------------------------- snow
def make_snow():
    albedo, height, rough = new_state()
    rng = random.Random("snow")
    H, S, V = 45, 0.07, 0.94  # matches sim BlockId::SNOW #f0ece0
    BR = 0.58
    for y in range(SIZE):
        for x in range(SIZE):
            dimple = math.sin(x * 0.20 + 0.4) * math.cos(y * 0.18 + 1.0) * 0.025
            v_ref = V + dimple
            v = v_ref + jitter(rng, 0, 0.02)
            s = S + jitter(rng, 0, 0.02)
            h = H
            rr = BR
            if rng.random() < 0.05:
                v = min(1.0, v + 0.05)  # sparkle fleck — icy, smoother
                s = 0.03
                rr -= 0.18
            elif rng.random() < 0.03:
                h = 205  # rare cool glint
                s = 0.05
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# ------------------------------------------------------------------- water
def make_water():
    albedo, height, rough = new_state()
    rng = random.Random("water")
    H, S, V = 205, 0.55, 0.55
    BR = 0.10
    for y in range(SIZE):
        wave = math.sin(y * 0.22 + math.sin(y * 0.08) * 1.5) * 0.5 + 0.5
        for x in range(SIZE):
            ripple = math.sin(x * 0.3 + y * 0.15) * 0.02
            v_ref = V + wave * 0.10
            v = v_ref + ripple + jitter(rng, 0, 0.02)
            s = S - wave * 0.05 + jitter(rng, 0, 0.03)
            h = H + jitter(rng, 0, 4)
            rr = BR
            if rng.random() < 0.04:
                v += 0.22  # sun-glint highlight streak, near-mirror
                s -= 0.15
                rr = 0.03
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr, rough_override=rr)
    return albedo, height, rough


# ------------------------------------------------------------------- metal
def make_metal():
    albedo, height, rough = new_state()
    rng = random.Random("metal")
    H, S, V = 212, 0.06, 0.62
    BR = 0.30
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            brushed = math.sin((x - y) * 0.35) * 0.05 + math.sin((x - y) * 1.4) * 0.015
            v = v_ref + brushed + jitter(rng, 0, 0.015)
            s = S + jitter(rng, 0, 0.02)
            rr = BR - brushed * 1.2
            rivet = (x % 20 in (9, 10)) and (y % 20 in (9, 10))
            if rivet:
                v -= 0.20
                s += 0.04
                rr = 0.55
            emit(albedo, height, rough, x, y, H, s, v, v_ref, rr, rough_override=rr)
    return albedo, height, rough


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


def build_before_after_sheet():
    cols = 2
    rows = len(MATERIALS)
    cell = 256  # display size for both before(16px*16) and after(64px*4)
    label_h = 22
    pad = 8
    W = cols * (cell + pad) + pad
    Hh = rows * (cell + label_h + pad) + pad
    sheet = Image.new("RGB", (W, Hh), (26, 27, 31))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 15)
        hdr_font = ImageFont.truetype("arialbd.ttf", 15)
    except Exception:
        font = ImageFont.load_default()
        hdr_font = font

    draw.text((pad, 2), "BEFORE (16x16)", fill=(200, 200, 210), font=hdr_font)
    draw.text((pad + cell + pad, 2), "AFTER (64x64 PBR albedo)", fill=(210, 230, 210), font=hdr_font)

    for i, (name, _) in enumerate(MATERIALS):
        y0 = pad + label_h + i * (cell + label_h + pad)
        before = Image.open(BACKUP_DIR / f"{name}.png").convert("RGB")
        before_big = before.resize((cell, cell), Image.NEAREST)
        after = Image.open(OUT_DIR / f"{name}.png").convert("RGB")
        after_big = after.resize((cell, cell), Image.NEAREST)
        x0 = pad
        sheet.paste(before_big, (x0, y0))
        draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        x1 = pad + cell + pad
        sheet.paste(after_big, (x1, y0))
        draw.rectangle([x1, y0, x1 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        draw.text((x0, y0 - label_h + 2), name, fill=(230, 230, 230), font=font)

    out_path = OUT_DIR / "contact_sheet.png"
    sheet.save(out_path)
    print(f"before/after contact sheet -> {out_path}  ({W}x{Hh})")


def build_pbr_sheet(avgs, roughs):
    cols = 3  # albedo, normal, roughness
    rows = len(MATERIALS)
    cell = 160
    label_h = 20
    pad = 6
    W = cols * (cell + pad) + pad
    Hh = rows * (cell + label_h + pad) + pad + label_h
    sheet = Image.new("RGB", (W, Hh), (26, 27, 31))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 13)
        hdr_font = ImageFont.truetype("arialbd.ttf", 14)
    except Exception:
        font = ImageFont.load_default()
        hdr_font = font

    headers = ["albedo", "normal", "roughness"]
    for c, hname in enumerate(headers):
        draw.text((pad + c * (cell + pad), 2), hname, fill=(210, 210, 220), font=hdr_font)

    for i, (name, _) in enumerate(MATERIALS):
        y0 = label_h + pad + i * (cell + label_h + pad)
        files = [f"{name}.png", f"{name}_n.png", f"{name}_r.png"]
        for c, fn in enumerate(files):
            x0 = pad + c * (cell + pad)
            tile = Image.open(OUT_DIR / fn).convert("RGB").resize((cell, cell), Image.NEAREST)
            sheet.paste(tile, (x0, y0))
            draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        hexcode = avgs[name]
        rr = roughs[name]
        draw.text((pad, y0 + cell + 2), f"{name}  avg {hexcode}  rough~{rr:.2f}", fill=(230, 230, 230), font=font)

    out_path = OUT_DIR / "contact_sheet_pbr.png"
    sheet.save(out_path)
    print(f"PBR triplet contact sheet -> {out_path}  ({W}x{Hh})")


def main():
    avgs = {}
    rough_means = {}
    for name, fn in MATERIALS:
        albedo, height, rough = fn()
        assert albedo.size == (SIZE, SIZE)
        save(albedo, name)
        n_img = normal_from_height(height)
        save(n_img, name, "_n")
        r_img = roughness_img(rough)
        save(r_img, name, "_r")
        hexcode, _ = avg_hex(albedo)
        avgs[name] = hexcode
        flat = [v for row in rough for v in row]
        rough_means[name] = sum(flat) / len(flat)
        print(f"{name:16s} 64x64 albedo {hexcode}  +_n +_r  rough~{rough_means[name]:.2f}")

    build_before_after_sheet()
    build_pbr_sheet(avgs, rough_means)


if __name__ == "__main__":
    main()
