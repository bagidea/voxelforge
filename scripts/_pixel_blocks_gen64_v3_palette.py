"""
Monanisa / art lane — palette-breadth + micro-detail polish pass (2026-08-18).

Director's brief: art-gap-vs-ceo-ref-2026-08-18.md G6+G7 —
  G6 palette breadth: 6/36 hue bins used vs REF's 15/36 (hue entropy 2.16 vs 3.90 bits)
  G7 detail per area: Sobel mean 30.8 vs REF's 58.6 (53%)

This is NOT a re-author from scratch. It loads the existing 19-material 64x64
PBR set produced by `_pixel_blocks_gen64.py` (imported as a module, its
make_* functions called directly so the base hue targets in PALETTE.md do not
move) and layers a SECOND deterministic pass of material-appropriate accent
detail on top of each one:

  - Secondary hues that are actually a different hue, not a brightness/value
    tweak of the same one: lichen/moss (~90-100deg green) on bark/roof/stone/
    dirt/brick/floorboards, rust/oxidation (~20-30deg orange) on metal/glass
    frame/stone iron-cramps, verdigris/patina (~150deg blue-green) on
    lamp-cage/metal, blue-grey clay/mineral/nail-head flecks (~200-210deg) on
    dirt/sand/oak_planks, warm golden sun-glint (~45deg, was previously the
    SAME hue as the water it sat on) on water, small red-berry/gold-leaf-edge
    accents on the leaf canopy, tiny wildflower speckle on grass, dark basalt
    grain + shell fragments on sand/red_sand, an occasional blue-black
    "clinker" brick.
  - Each accent is an irregular (angle-wobbled, not circular) organic blob,
    confined with a per-material exclude predicate to where it would
    plausibly occur (mortar cracks, grooves, cap lines, frame pixels, shaded
    dimples) rather than scattered uniformly — avoids the "just noise"
    failure G7 explicitly calls out (edge density that doesn't read as real
    detail).
  - Every accent pixel updates albedo AND the height/roughness fields in the
    same blended write, so normal/roughness regenerate from what's actually
    drawn (same `emit`-family convention as the base pass), not decorrelated
    noise.
  - All placement RNGs are seeded per-material-per-accent-name, so re-running
    this script reproduces byte-identical output.

Old (pre-polish) 64x64 set backed up verbatim at
assets/textures/blocks_64px_prepolish_backup/ before this script ever wrote
a pixel (see backup-procedural-asset skill). Same tile_px=64, same file
names, same atlas.json structure — no manifest change needed.
"""

import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402

SIZE = base.SIZE
OUT_DIR = base.OUT_DIR
BEFORE_DIR = base.ROOT / "assets" / "textures" / "blocks_64px_prepolish_backup"
clamp01 = base.clamp01
hsv = base.hsv
jitter = base.jitter
emit = base.emit


# --------------------------------------------------------------- accent core
def paint_accent(albedo, height, rough, x, y, h, s, v, target_height, target_rough, blend):
    old_rgb = albedo.getpixel((x, y))
    new_rgb = hsv(h, s, v)
    mixed = tuple(round(old_rgb[i] * (1 - blend) + new_rgb[i] * blend) for i in range(3)) + (255,)
    albedo.putpixel((x, y), mixed)
    height[y][x] = clamp01(height[y][x] * (1 - blend) + target_height * blend)
    rough[y][x] = clamp01(rough[y][x] * (1 - blend) + target_rough * blend)


def scatter_blobs(rng, n, r_range, exclude=None, tries_mul=25):
    """Deterministically place up to n organic blob centers, honoring `exclude(cx, cy) -> bool skip`."""
    placed = []
    tries = 0
    while len(placed) < n and tries < n * tries_mul:
        tries += 1
        cx = rng.uniform(0, SIZE)
        cy = rng.uniform(0, SIZE)
        if exclude is not None and exclude(cx, cy):
            continue
        placed.append((cx, cy, rng.uniform(*r_range), rng.uniform(0.25, 0.45),
                        rng.choice([3, 4, 5]), rng.uniform(0, math.tau)))
    return placed


def paint_blobs(rng, albedo, height, rough, blobs, h, s_range, v, target_height, target_rough,
                 max_blend=1.0, h_jitter=4):
    for (cx, cy, base_r, wobble, freq, phase) in blobs:
        pad = base_r * (1 + wobble) + 1
        x0, x1 = int(math.floor(cx - pad)), int(math.ceil(cx + pad))
        y0, y1 = int(math.floor(cy - pad)), int(math.ceil(cy + pad))
        for yy in range(y0, y1 + 1):
            py = yy % SIZE
            for xx in range(x0, x1 + 1):
                px = xx % SIZE
                ddx, ddy = xx - cx, yy - cy
                d = math.hypot(ddx, ddy)
                if d < 1e-6:
                    ang = 0.0
                else:
                    ang = math.atan2(ddy, ddx)
                # A single sin(freq*ang) is a clean n-petal rose curve -- at
                # this module's wobble/max_blend it reads as a pointed star,
                # not an organic blob (2026-08-19/20 star-splat bug). Summing
                # 4 octaves at incommensurate frequencies (no small-integer
                # ratio between them) breaks the rotational symmetry into an
                # irregular silhouette instead of N evenly-spaced points, all
                # derived from the same (freq, phase) so no new RNG state is
                # needed -- seeded output stays reproducible and the
                # hand-authored 6-tuple blobs elsewhere keep working.
                wobble_n = (
                    0.36 * math.sin(freq * ang + phase)
                    + 0.34 * math.sin((freq * 1.37 + 0.5) * ang + phase * 1.9 + 0.8)
                    + 0.20 * math.sin((freq * 2.1 + 1.1) * ang + phase * 0.6 + 2.1)
                    + 0.10 * math.sin((freq * 3.3 + 0.2) * ang + phase * 2.7 + 1.4)
                )
                r = base_r * (1 + 0.8 * wobble * wobble_n)
                if d > r:
                    continue
                edge = clamp01((r - d) / max(r * 0.35, 0.001)) * max_blend
                hh = h + jitter(rng, 0, h_jitter)
                ss = rng.uniform(*s_range)
                vv = v + jitter(rng, 0, 0.04)
                paint_accent(albedo, height, rough, px, py, hh, ss, vv, target_height, target_rough, edge)


def regrain(rng, albedo, height, rough, angle, freq, amp, h_base, s_amp=0.0):
    """Add directional streak grain (already-hue-correct materials, no new hue) for extra Sobel detail."""
    ca, sa = math.cos(angle), math.sin(angle)
    for y in range(SIZE):
        for x in range(SIZE):
            proj = x * ca + y * sa
            g = math.sin(proj * freq) * amp + math.sin(proj * freq * 2.7 + 1.3) * amp * 0.4
            old_rgb = albedo.getpixel((x, y))
            import colorsys
            r, gg, b = (c / 255 for c in old_rgb[:3])
            hh, ss, vv = colorsys.rgb_to_hsv(r, gg, b)
            vv = clamp01(vv + g)
            ss = clamp01(ss + g * s_amp)
            albedo.putpixel((x, y), hsv(hh * 360, ss, vv))
            height[y][x] = clamp01(height[y][x] + g * 0.9)
            rough[y][x] = clamp01(rough[y][x] - g * 0.3)


# ------------------------------------------------------------ per-material
def polish_oak_planks(albedo, height, rough):
    rng = base.__dict__.setdefault("_rng_cache", {})
    r = __import__("random").Random("oak_planks-nails")
    plank_w = 16
    for plank in range(4):
        col = plank * plank_w + plank_w // 2
        for row_i in range(3):
            y = 6 + row_i * 24 + int(r.uniform(-1, 1))
            x = col + int(r.uniform(-2, 2))
            blobs = scatter_blobs(r, 1, (1.1, 1.5))
            paint_blobs(r, albedo, height, rough, [(x, y, 1.2, 0.15, 4, 0.0)],
                        h=208, s_range=(0.05, 0.12), v=0.26, target_height=0.30, target_rough=0.42, h_jitter=6)
    return albedo, height, rough


def polish_log_side(albedo, height, rough):
    r = __import__("random").Random("oak_log_side-lichen")
    blobs = scatter_blobs(r, 5, (2.5, 5.0))
    paint_blobs(r, albedo, height, rough, blobs, h=96, s_range=(0.28, 0.42), v=0.42,
                target_height=0.58, target_rough=0.82, max_blend=0.75)
    return albedo, height, rough


def polish_log_top(albedo, height, rough):
    r = __import__("random").Random("oak_log_top-spalt")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2

    def near_rim(cx_, cy_):
        return math.hypot(cx_ - cx, cy_ - cy) < 20

    blobs = scatter_blobs(r, 4, (2.0, 3.6), exclude=near_rim)
    paint_blobs(r, albedo, height, rough, blobs, h=200, s_range=(0.12, 0.20), v=0.50,
                target_height=0.42, target_rough=0.78, max_blend=0.6)
    blobs2 = scatter_blobs(r, 6, (1.4, 2.6), exclude=lambda x, y: math.hypot(x - cx, y - cy) < 27)
    paint_blobs(r, albedo, height, rough, blobs2, h=98, s_range=(0.25, 0.35), v=0.40,
                target_height=0.56, target_rough=0.80, max_blend=0.7)
    return albedo, height, rough


def polish_stone_bricks(albedo, height, rough):
    r = __import__("random").Random("stone_bricks-weather")
    brick_h = 16

    def is_mortar(x, y):
        row = int(y) // brick_h
        offset = 16 if row % 2 else 0
        in_row = int(y) % brick_h
        bx = (int(x) + offset) % SIZE
        return in_row < 2 or bx % 32 < 2

    moss = scatter_blobs(r, 6, (1.6, 3.0), exclude=lambda x, y: not is_mortar(x, y))
    paint_blobs(r, albedo, height, rough, moss, h=97, s_range=(0.30, 0.42), v=0.36,
                target_height=0.62, target_rough=0.85, max_blend=0.8)
    rust = scatter_blobs(r, 3, (1.2, 2.2), exclude=lambda x, y: not is_mortar(x, y))
    paint_blobs(r, albedo, height, rough, rust, h=24, s_range=(0.45, 0.60), v=0.42,
                target_height=0.40, target_rough=0.70, max_blend=0.7)
    return albedo, height, rough


def polish_sand(albedo, height, rough):
    r = __import__("random").Random("sand-grains")
    shells = scatter_blobs(r, 8, (0.9, 1.6))
    paint_blobs(r, albedo, height, rough, shells, h=20, s_range=(0.05, 0.14), v=0.93,
                target_height=0.62, target_rough=0.55, max_blend=0.85, h_jitter=10)
    mineral = scatter_blobs(r, 10, (0.6, 1.1))
    paint_blobs(r, albedo, height, rough, mineral, h=30, s_range=(0.10, 0.20), v=0.22,
                target_height=0.34, target_rough=0.70, max_blend=0.9)
    seaglass = scatter_blobs(r, 2, (0.8, 1.2))
    paint_blobs(r, albedo, height, rough, seaglass, h=172, s_range=(0.30, 0.45), v=0.55,
                target_height=0.55, target_rough=0.30, max_blend=0.9)
    return albedo, height, rough


def polish_grass_top(albedo, height, rough):
    r = __import__("random").Random("grass_top-wildflower")
    yellow = scatter_blobs(r, 6, (0.9, 1.5))
    paint_blobs(r, albedo, height, rough, yellow, h=50, s_range=(0.55, 0.75), v=0.85,
                target_height=0.62, target_rough=0.55, max_blend=0.9)
    white = scatter_blobs(r, 3, (0.8, 1.2))
    paint_blobs(r, albedo, height, rough, white, h=40, s_range=(0.03, 0.08), v=0.92,
                target_height=0.60, target_rough=0.58, max_blend=0.85)
    return albedo, height, rough


def polish_grass_side(albedo, height, rough):
    r = __import__("random").Random("grass_side-clay")
    cap = [4 * base.SCALE + (2 if (x // base.SCALE) in (2, 5, 9, 13) else 0) + int(2 * math.sin(x * 0.4))
           for x in range(SIZE)]

    def in_dirt(x, y):
        xi = int(x) % SIZE
        return y <= cap[xi] + 2

    clay = scatter_blobs(r, 5, (1.4, 2.4), exclude=in_dirt)
    paint_blobs(r, albedo, height, rough, clay, h=202, s_range=(0.14, 0.22), v=0.34,
                target_height=0.40, target_rough=0.68, max_blend=0.75)
    moss = scatter_blobs(r, 3, (1.0, 1.8), exclude=lambda x, y: not (cap[int(x) % SIZE] <= y <= cap[int(x) % SIZE] + 4))
    paint_blobs(r, albedo, height, rough, moss, h=94, s_range=(0.35, 0.45), v=0.40,
                target_height=0.58, target_rough=0.82, max_blend=0.7)
    return albedo, height, rough


def polish_leaves(albedo, height, rough):
    r = __import__("random").Random("leaves-berries")
    berries = scatter_blobs(r, 5, (0.9, 1.4))
    paint_blobs(r, albedo, height, rough, berries, h=355, s_range=(0.55, 0.70), v=0.55,
                target_height=0.64, target_rough=0.45, max_blend=0.9, h_jitter=6)
    autumn = scatter_blobs(r, 7, (1.6, 2.8))
    paint_blobs(r, albedo, height, rough, autumn, h=46, s_range=(0.45, 0.60), v=0.58,
                target_height=0.60, target_rough=0.62, max_blend=0.55, h_jitter=8)
    return albedo, height, rough


def polish_roof_tile(albedo, height, rough):
    r = __import__("random").Random("roof_tile-moss")
    tile_h = 16

    def edge_groove(x, y):
        row = int(y) // tile_h
        offset = 8 if row % 2 else 0
        in_row = int(y) % tile_h
        bx = (int(x) + offset) % SIZE
        return not (in_row < 3 or bx % 16 < 3)

    moss = scatter_blobs(r, 6, (1.5, 2.6), exclude=edge_groove)
    paint_blobs(r, albedo, height, rough, moss, h=95, s_range=(0.35, 0.48), v=0.34,
                target_height=0.58, target_rough=0.85, max_blend=0.75)
    lime = scatter_blobs(r, 3, (1.0, 1.8), exclude=edge_groove)
    paint_blobs(r, albedo, height, rough, lime, h=45, s_range=(0.06, 0.12), v=0.72,
                target_height=0.44, target_rough=0.60, max_blend=0.6)
    return albedo, height, rough


def polish_glass(albedo, height, rough):
    r = __import__("random").Random("glass-rust")

    def not_frame(x, y):
        xi, yi = int(x) % SIZE, int(y) % SIZE
        border = xi < 3 or xi >= SIZE - 3 or yi < 3 or yi >= SIZE - 3
        mullion = (xi % 32 < 2) or (yi % 32 < 2)
        return not (border or mullion)

    rust = scatter_blobs(r, 3, (1.0, 1.8), exclude=not_frame)
    paint_blobs(r, albedo, height, rough, rust, h=24, s_range=(0.45, 0.60), v=0.38,
                target_height=0.36, target_rough=0.68, max_blend=0.7)
    return albedo, height, rough


def polish_clay_plaster(albedo, height, rough):
    r = __import__("random").Random("clay_plaster-damp")
    damp = scatter_blobs(r, 4, (2.0, 3.5), exclude=lambda x, y: y < SIZE * 0.55)
    paint_blobs(r, albedo, height, rough, damp, h=100, s_range=(0.18, 0.26), v=0.62,
                target_height=0.44, target_rough=0.72, max_blend=0.5)
    ochre = scatter_blobs(r, 2, (1.4, 2.4))
    paint_blobs(r, albedo, height, rough, ochre, h=40, s_range=(0.20, 0.30), v=0.70,
                target_height=0.48, target_rough=0.62, max_blend=0.5)
    return albedo, height, rough


def polish_floorboards(albedo, height, rough):
    r = __import__("random").Random("floorboards-algae")
    board_h = 16

    def near_seam(x, y):
        within = int(y) % board_h
        return within < board_h - 4

    algae = scatter_blobs(r, 5, (1.6, 2.8), exclude=near_seam)
    paint_blobs(r, albedo, height, rough, algae, h=98, s_range=(0.30, 0.42), v=0.38,
                target_height=0.56, target_rough=0.80, max_blend=0.75)
    r2 = __import__("random").Random("floorboards-nails")
    for board in range(4):
        y = board * 16 + 8
        for x in (10, 32, 54):
            paint_blobs(r2, albedo, height, rough, [(x, y, 1.1, 0.15, 4, 0.0)],
                        h=208, s_range=(0.05, 0.10), v=0.24, target_height=0.30, target_rough=0.42, h_jitter=6)
    return albedo, height, rough


def polish_dirt(albedo, height, rough):
    r = __import__("random").Random("dirt-clay-moss")
    clay = scatter_blobs(r, 5, (1.4, 2.4))
    paint_blobs(r, albedo, height, rough, clay, h=201, s_range=(0.14, 0.22), v=0.33,
                target_height=0.42, target_rough=0.66, max_blend=0.75)
    moss = scatter_blobs(r, 4, (1.2, 2.0))
    paint_blobs(r, albedo, height, rough, moss, h=95, s_range=(0.32, 0.42), v=0.36,
                target_height=0.58, target_rough=0.82, max_blend=0.65)
    return albedo, height, rough


def polish_brick(albedo, height, rough):
    r = __import__("random").Random("brick-clinker")
    brick_h = 16
    clinker_units = set()
    for row in range(4):
        offset = 16 if row % 2 else 0
        for col in range(2):
            if r.random() < 0.16:
                clinker_units.add((row, col, offset))
    for y in range(SIZE):
        row = y // brick_h
        offset = 16 if row % 2 else 0
        in_row = y % brick_h
        if in_row < 2:
            continue
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            if bx % 32 < 2:
                continue
            col = bx // 32
            if (row, col, offset) in clinker_units:
                blend = 0.55 + jitter(r, 0, 0.1)
                paint_accent(albedo, height, rough, x, y, 220, 0.10, 0.28, 0.36, 0.55, clamp01(blend))
    moss = scatter_blobs(r, 4, (1.2, 2.2),
                          exclude=lambda x, y: (int(y) % brick_h) >= 2 and ((int(x) + (16 if (int(y) // brick_h) % 2 else 0)) % 32) >= 2)
    paint_blobs(r, albedo, height, rough, moss, h=96, s_range=(0.30, 0.40), v=0.36,
                target_height=0.56, target_rough=0.82, max_blend=0.7)
    return albedo, height, rough


def polish_lamp(albedo, height, rough):
    r = __import__("random").Random("lamp-patina")

    def not_corner(x, y):
        xi, yi = int(x) % SIZE, int(y) % SIZE
        return not (xi < 6 or xi >= SIZE - 6 or yi < 6 or yi >= SIZE - 6)

    patina = scatter_blobs(r, 5, (1.2, 2.0), exclude=not_corner)
    paint_blobs(r, albedo, height, rough, patina, h=152, s_range=(0.35, 0.50), v=0.38,
                target_height=0.44, target_rough=0.62, max_blend=0.8)
    return albedo, height, rough


def polish_red_sand(albedo, height, rough):
    r = __import__("random").Random("red_sand-grains")
    mineral = scatter_blobs(r, 8, (0.6, 1.1))
    paint_blobs(r, albedo, height, rough, mineral, h=15, s_range=(0.10, 0.20), v=0.20,
                target_height=0.34, target_rough=0.70, max_blend=0.9)
    olivine = scatter_blobs(r, 4, (0.7, 1.2))
    paint_blobs(r, albedo, height, rough, olivine, h=110, s_range=(0.30, 0.42), v=0.38,
                target_height=0.56, target_rough=0.72, max_blend=0.85, h_jitter=8)
    shell = scatter_blobs(r, 3, (0.8, 1.3))
    paint_blobs(r, albedo, height, rough, shell, h=25, s_range=(0.08, 0.16), v=0.86,
                target_height=0.60, target_rough=0.52, max_blend=0.8)
    return albedo, height, rough


def polish_snow(albedo, height, rough):
    r = __import__("random").Random("snow-mud")
    mud = scatter_blobs(r, 2, (1.0, 1.6))
    paint_blobs(r, albedo, height, rough, mud, h=32, s_range=(0.14, 0.22), v=0.55,
                target_height=0.40, target_rough=0.75, max_blend=0.35)
    regrain(r, albedo, height, rough, angle=0.9, freq=0.35, amp=0.02, h_base=45)
    return albedo, height, rough


def polish_water(albedo, height, rough):
    r = __import__("random").Random("water-repolish")
    glint = scatter_blobs(r, 6, (1.6, 2.8))
    paint_blobs(r, albedo, height, rough, glint, h=46, s_range=(0.35, 0.55), v=0.86,
                target_height=0.70, target_rough=0.06, max_blend=0.85, h_jitter=6)
    algae = scatter_blobs(r, 3, (1.4, 2.2))
    paint_blobs(r, albedo, height, rough, algae, h=150, s_range=(0.30, 0.42), v=0.42,
                target_height=0.52, target_rough=0.30, max_blend=0.4)
    return albedo, height, rough


def polish_metal(albedo, height, rough):
    r = __import__("random").Random("metal-oxidation")
    rust = scatter_blobs(r, 4, (1.4, 2.4))
    paint_blobs(r, albedo, height, rough, rust, h=24, s_range=(0.45, 0.60), v=0.40,
                target_height=0.36, target_rough=0.72, max_blend=0.75)
    patina = scatter_blobs(r, 3, (1.2, 2.0))
    paint_blobs(r, albedo, height, rough, patina, h=155, s_range=(0.30, 0.42), v=0.42,
                target_height=0.56, target_rough=0.68, max_blend=0.7)
    return albedo, height, rough


MATERIALS = [
    ("oak_planks", base.make_oak_planks, polish_oak_planks),
    ("oak_log_side", base.make_log_side, polish_log_side),
    ("oak_log_top", base.make_log_top, polish_log_top),
    ("stone_bricks", base.make_stone_bricks, polish_stone_bricks),
    ("sand", base.make_sand, polish_sand),
    ("grass_top", base.make_grass_top, polish_grass_top),
    ("grass_side", base.make_grass_side, polish_grass_side),
    ("leaves", base.make_leaves, polish_leaves),
    ("roof_tile", base.make_roof_tile, polish_roof_tile),
    ("glass", base.make_glass, polish_glass),
    ("clay_plaster", base.make_clay_plaster, polish_clay_plaster),
    ("floorboards", base.make_floorboards, polish_floorboards),
    ("dirt", base.make_dirt, polish_dirt),
    ("brick", base.make_brick, polish_brick),
    ("lamp", base.make_lamp, polish_lamp),
    ("red_sand", base.make_red_sand, polish_red_sand),
    ("snow", base.make_snow, polish_snow),
    ("water", base.make_water, polish_water),
    ("metal", base.make_metal, polish_metal),
]


def save(img, name, suffix=""):
    path = OUT_DIR / f"{name}{suffix}.png"
    img.save(path)
    return path


def avg_hex(img):
    px = list(img.convert("RGB").getdata())
    avg = tuple(round(sum(c[i] for c in px) / len(px)) for i in range(3))
    return f"#{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}", avg


def tile2x2(img):
    w, h = img.size
    out = Image.new("RGB", (w * 2, h * 2))
    for j in range(2):
        for i in range(2):
            out.paste(img.convert("RGB"), (i * w, j * h))
    return out


def build_before_after_sheet():
    cols = 2
    rows = len(MATERIALS)
    cell = 220
    label_h = 22
    pad = 8
    W = cols * (cell + pad) + pad
    Hh = rows * (cell + label_h + pad) + pad
    sheet = Image.new("RGB", (W, Hh), (26, 27, 31))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 14)
        hdr_font = ImageFont.truetype("arialbd.ttf", 15)
    except Exception:
        font = ImageFont.load_default()
        hdr_font = font

    draw.text((pad, 2), "BEFORE (pre-polish 64x64)", fill=(200, 200, 210), font=hdr_font)
    draw.text((pad + cell + pad, 2), "AFTER (palette+detail polish)", fill=(210, 230, 210), font=hdr_font)

    for i, (name, _, _) in enumerate(MATERIALS):
        y0 = pad + label_h + i * (cell + label_h + pad)
        before = Image.open(BEFORE_DIR / f"{name}.png").convert("RGB")
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

    out_path = OUT_DIR / "contact_sheet_v3_palette.png"
    sheet.save(out_path)
    print(f"before/after palette-polish contact sheet -> {out_path}  ({W}x{Hh})")


def build_tiled_check_sheet():
    cols = 4
    rows = math.ceil(len(MATERIALS) / cols)
    cell = 160
    label_h = 18
    pad = 6
    W = cols * (cell + pad) + pad
    Hh = rows * (cell + label_h + pad) + pad
    sheet = Image.new("RGB", (W, Hh), (26, 27, 31))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 12)
    except Exception:
        font = ImageFont.load_default()
    for i, (name, _, _) in enumerate(MATERIALS):
        col, row = i % cols, i // cols
        x0 = pad + col * (cell + pad)
        y0 = pad + label_h + row * (cell + label_h + pad)
        img = Image.open(OUT_DIR / f"{name}.png")
        tiled = tile2x2(img).resize((cell, cell), Image.NEAREST)
        sheet.paste(tiled, (x0, y0))
        draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        draw.text((x0, y0 - label_h + 2), name, fill=(220, 220, 220), font=font)
    out_path = OUT_DIR / "_v3_2x2_tile_check.png"
    sheet.save(out_path)
    print(f"2x2 tiled-zoom seam check -> {out_path}  ({W}x{Hh})")
    return out_path


def main():
    avgs = {}
    for name, make_fn, polish_fn in MATERIALS:
        albedo, height, rough = make_fn()
        albedo, height, rough = polish_fn(albedo, height, rough)
        assert albedo.size == (SIZE, SIZE)
        save(albedo, name)
        n_img = base.normal_from_height(height)
        save(n_img, name, "_n")
        r_img = base.roughness_img(rough)
        save(r_img, name, "_r")
        hexcode, _ = avg_hex(albedo)
        avgs[name] = hexcode
        print(f"{name:16s} polished  avg {hexcode}")

    build_before_after_sheet()
    build_tiled_check_sheet()


if __name__ == "__main__":
    main()
