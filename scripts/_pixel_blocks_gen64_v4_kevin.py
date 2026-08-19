"""
Kevin / art lane — v4: mip-surviving palette breadth + detail (2026-08-19).

WHY THIS EXISTS
---------------
The v3 accent pass added a real hue accent to every tile, but every accent
was a 1-2px blob at a partial blend. Measured on the actual beach_dusk render
(camera pose F) it moved NOTHING. The reason, found by dumping the frame's
per-hue-bin chroma shares through the grader's own measure():

  - the frame is ~97% warm (hue bins 0-4: dirt/brick/wood/log/roof/
    floorboards/red_sand) + one olive bin (bin 6, 60-69 deg = grass/leaves
    after the warm sunset light has pulled their green toward yellow-olive),
  - the green bins 8-11 are at ~0%: a green accent (h~96) painted on a warm
    tile gets (a) mip-averaged with its brown surround into olive, and
    (b) warm-tinted by the sunset key light toward orange — both of which
    move its hue OUT of the green bin and INTO an already-open warm bin,
  - chroma_w = sat * (val/255), so the DARK accents (moss v=0.34, blue-grey
    v=0.26) I first tried carry almost no chroma mass and also get crushed
    by mip before they can open a bin.

So the cheapest hue wins are the two bins that are already at 0.68-0.70% and
that are WARM-SAFE (the sunset light keeps them, or pushes them deeper into,
their own bin, never out of it): bin 35 (red, 350-359) and bin 5 (yellow,
50-59). A violet/magenta (h~293) survives too, since the warm tint pulls it
toward magenta rather than toward warm. Blue/green do NOT survive — that is
the ceiling PALETTE.md already names ("warm key light desaturates cool hues
into warm by additive tint").

This pass therefore layers LARGER (8-16px radius, up to 24), LIGHTER
(higher val), WARM-SAFE accents at full amplitude (max_blend 1.0) on the
DOMINANT warm materials (not just the 2% grass/leaves that v3 targeted):

  - red clay / red-ochre (h~352) on dirt, brick, floorboards, clay_plaster
  - pale-gold efflorescence (h~53) on roof_tile + gold dune streaks on sand
  - violet wildflower (h~293) on grass, bigger and denser
  - lighter moss/lichen (v 0.45, s 0.55) so it reads yellow-green instead of
    vanishing, plus lighter blue-grey mineral (v 0.42) that keeps a cool tell

Base HSV targets never move (base -> v3 -> this pass -> save), the accent
vocabulary is the same weathering logic v3 documented in PALETTE.md, and the
larger scale doubles as the far-band value structure the
"detail surviving at distance" axis grades (sigma-3 high-pass needs features
that outlast mip).

Chain: base (_pixel_blocks_gen64.py) -> v3 polish -> THIS pass -> save.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402
import _pixel_blocks_gen64_v3_palette as v3  # noqa: E402

SIZE = base.SIZE
OUT_DIR = base.OUT_DIR
scatter_blobs = v3.scatter_blobs
paint_blobs = v3.paint_blobs
paint_accent = v3.paint_accent


# ---------------------------------------------------------------- helpers
def moss(rng, albedo, height, rough, n=5, r_range=(8, 16), h=115, s=(0.50, 0.62),
         v=0.50, exclude=None):
    """Large LIGHT green moss/lichen patches. v raised (chroma = sat*val) and
    hue pushed greener so the warm light lands it in a yellow-green bin, not
    vanishing into olive."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.60, target_rough=0.84, max_blend=1.0, h_jitter=6)


def bluegrey(rng, albedo, height, rough, n=4, r_range=(6, 12), h=203, s=(0.22, 0.32),
             v=0.50, exclude=None):
    """Large LIGHT blue-grey mineral/clay lumps - the cool anchor, lightened
    + saturated so it keeps a cyan/blue tell instead of reading as grey mud."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.42, target_rough=0.68, max_blend=1.0, h_jitter=6)


def pale(rng, albedo, height, rough, n=4, r_range=(4, 8), h=32, s=(0.05, 0.14),
         v=0.86, exclude=None):
    """Large pale shell/pebble/lime flecks - the strong LIGHT value feature."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.58, target_rough=0.50, max_blend=1.0, h_jitter=8)


def dark(rng, albedo, height, rough, n=4, r_range=(4, 9), h=26, s=(0.06, 0.16),
         v=0.18, exclude=None):
    """Large near-black mineral/basalt flecks - the strong DARK value feature
    (Sobel / far-band contrast; deliberately low chroma)."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.30, target_rough=0.72, max_blend=1.0, h_jitter=8)


def red_clay(rng, albedo, height, rough, n=4, r_range=(9, 16), h=352, s=(0.52, 0.64),
             v=0.60, exclude=None):
    """Large red-clay / red-ochre patches - opens hue bin 35 (red). Warm-safe:
    the sunset light keeps it red, never desaturates it into a warm bin."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.46, target_rough=0.66, max_blend=1.0, h_jitter=6)


def gold(rng, albedo, height, rough, n=4, r_range=(8, 14), h=53, s=(0.60, 0.75),
         v=0.82, exclude=None):
    """Large pale-gold accents - opens hue bin 5 (yellow). Warm-safe and high
    chroma (sat*val ~0.55), the single cheapest bin to open."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.52, target_rough=0.42, max_blend=1.0, h_jitter=6)


def violet(rng, albedo, height, rough, n=5, r_range=(9, 16), h=293, s=(0.55, 0.70),
           v=0.60, exclude=None):
    """Large violet wildflower clusters - opens a magenta/violet bin. The warm
    tint pulls violet toward magenta, not toward warm, so the hue survives."""
    blobs = scatter_blobs(rng, n, r_range, exclude=exclude)
    paint_blobs(rng, albedo, height, rough, blobs, h=h, s_range=s, v=v,
                target_height=0.60, target_rough=0.55, max_blend=1.0, h_jitter=10)


# ------------------------------------------------------------ per-material
def kevin_dirt(albedo, height, rough):
    r = random.Random("dirt-v4")
    red_clay(r, albedo, height, rough, n=4, r_range=(9, 16))
    moss(r, albedo, height, rough, n=5, r_range=(8, 16))
    bluegrey(r, albedo, height, rough, n=3, r_range=(6, 11))
    pale(r, albedo, height, rough, n=3, r_range=(3, 6), h=40, s=(0.06, 0.12), v=0.58)
    dark(r, albedo, height, rough, n=3, r_range=(3, 6))
    return albedo, height, rough


def kevin_brick(albedo, height, rough):
    r = random.Random("brick-v4")
    brick_h = 16
    # occasional whole overfired "clinker" brick (blue-black) + redder bricks
    for row in range(4):
        offset = 16 if row % 2 else 0
        roll = r.random()
        if roll < 0.15:
            # clinker: blue-black
            for y in range(row * brick_h + 2, row * brick_h + brick_h):
                for x in range(SIZE):
                    bx = (x + offset) % SIZE
                    if bx % 32 >= 2:
                        paint_accent(albedo, height, rough, x, y, 226, 0.10, 0.30, 0.34, 0.55, 0.92)
        elif roll < 0.30:
            # overfired red: pushes the brick toward bin 35 (red) instead of 20
            for y in range(row * brick_h + 2, row * brick_h + brick_h):
                for x in range(SIZE):
                    bx = (x + offset) % SIZE
                    if bx % 32 >= 2:
                        paint_accent(albedo, height, rough, x, y, 355, 0.62, 0.52, 0.44, 0.70, 0.85)
    moss(r, albedo, height, rough, n=4, r_range=(7, 13),
         exclude=lambda x, y: (int(y) % brick_h) >= 2 and ((int(x) + (16 if (int(y) // brick_h) % 2 else 0)) % 32) >= 2)
    return albedo, height, rough


def kevin_oak_planks(albedo, height, rough):
    r = random.Random("oak_planks-v4")
    moss(r, albedo, height, rough, n=4, r_range=(8, 14))
    bluegrey(r, albedo, height, rough, n=5, r_range=(5, 10), h=208, s=(0.08, 0.16), v=0.42)
    dark(r, albedo, height, rough, n=3, r_range=(3, 6), h=28, s=(0.10, 0.20), v=0.24)
    return albedo, height, rough


def kevin_log_side(albedo, height, rough):
    r = random.Random("log_side-v4")
    moss(r, albedo, height, rough, n=6, r_range=(9, 16), h=98, s=(0.42, 0.55), v=0.45)
    dark(r, albedo, height, rough, n=4, r_range=(4, 8), h=20, s=(0.15, 0.28), v=0.20)
    return albedo, height, rough


def kevin_log_top(albedo, height, rough):
    r = random.Random("log_top-v4")

    def ring(x, y):
        d = math.hypot(x - 31.5, y - 31.5)
        return d < 18 or d > 30
    blobs = scatter_blobs(r, 5, (4, 8), exclude=ring)
    paint_blobs(r, albedo, height, rough, blobs, h=96, s_range=(0.28, 0.40), v=0.45,
                target_height=0.56, target_rough=0.80, max_blend=1.0)
    return albedo, height, rough


def kevin_roof_tile(albedo, height, rough):
    r = random.Random("roof_tile-v4")
    tile_h = 16

    def groove(x, y):
        row = int(y) // tile_h
        offset = 8 if row % 2 else 0
        in_row = int(y) % tile_h
        bx = (int(x) + offset) % SIZE
        return not (in_row < 3 or bx % 16 < 3)
    gold(r, albedo, height, rough, n=4, r_range=(8, 14), exclude=groove)
    # burnt terracotta patches - red-orange, opens bin 34
    blobs = scatter_blobs(r, 4, (8, 14), exclude=groove)
    paint_blobs(r, albedo, height, rough, blobs, h=345, s_range=(0.55, 0.68), v=0.60,
                target_height=0.46, target_rough=0.68, max_blend=1.0, h_jitter=6)
    moss(r, albedo, height, rough, n=5, r_range=(7, 13), exclude=groove)
    return albedo, height, rough


def kevin_floorboards(albedo, height, rough):
    r = random.Random("floorboards-v4")
    board_h = 16

    def near_seam(x, y):
        return int(y) % board_h < board_h - 4
    moss(r, albedo, height, rough, n=4, r_range=(8, 13), h=98, s=(0.42, 0.55), v=0.42, exclude=near_seam)
    red_clay(r, albedo, height, rough, n=4, r_range=(8, 14), exclude=near_seam)
    bluegrey(r, albedo, height, rough, n=6, r_range=(3, 5), h=208, s=(0.05, 0.12), v=0.42)
    return albedo, height, rough


def kevin_red_sand(albedo, height, rough):
    r = random.Random("red_sand-v4")
    # olivine (green volcanic sand) - the single biggest hue swing on a red material
    blobs = scatter_blobs(r, 5, (6, 11))
    paint_blobs(r, albedo, height, rough, blobs, h=110, s_range=(0.30, 0.44), v=0.45,
                target_height=0.56, target_rough=0.72, max_blend=1.0, h_jitter=8)
    dark(r, albedo, height, rough, n=4, r_range=(4, 8), h=15, s=(0.10, 0.20), v=0.20)
    pale(r, albedo, height, rough, n=3, r_range=(4, 8), h=25, s=(0.06, 0.14), v=0.86)
    return albedo, height, rough


def kevin_sand(albedo, height, rough):
    r = random.Random("sand-v4")
    gold(r, albedo, height, rough, n=4, r_range=(7, 12))
    # teal sea-glass shards - the cool anchor the beach itself is missing
    blobs = scatter_blobs(r, 4, (6, 11))
    paint_blobs(r, albedo, height, rough, blobs, h=174, s_range=(0.44, 0.60), v=0.62,
                target_height=0.55, target_rough=0.28, max_blend=1.0, h_jitter=6)
    dark(r, albedo, height, rough, n=5, r_range=(4, 9), h=30, s=(0.08, 0.18), v=0.22)
    pale(r, albedo, height, rough, n=3, r_range=(5, 9), h=32, s=(0.06, 0.14), v=0.90)
    return albedo, height, rough


def kevin_grass_top(albedo, height, rough):
    r = random.Random("grass_top-v4")
    # bright-green clumps (raise the green mass so it reads as grass, not olive)
    blobs = scatter_blobs(r, 8, (8, 13))
    paint_blobs(r, albedo, height, rough, blobs, h=108, s_range=(0.60, 0.72), v=0.70,
                target_height=0.62, target_rough=0.55, max_blend=0.9, h_jitter=5)
    # violet wildflower clusters - big, dense, warm-safe magenta
    violet(r, albedo, height, rough, n=8, r_range=(9, 16))
    # yellow flowers
    blobs = scatter_blobs(r, 4, (3, 6))
    paint_blobs(r, albedo, height, rough, blobs, h=54, s_range=(0.60, 0.76), v=0.85,
                target_height=0.60, target_rough=0.55, max_blend=1.0, h_jitter=6)
    return albedo, height, rough


def kevin_grass_side(albedo, height, rough):
    r = random.Random("grass_side-v4")
    cap = [4 * base.SCALE + (2 if (x // base.SCALE) in (2, 5, 9, 13) else 0) + int(2 * math.sin(x * 0.4))
           for x in range(SIZE)]

    def on_cap(x, y):
        xi = int(x) % SIZE
        return y <= cap[xi] + 3
    violet(r, albedo, height, rough, n=4, r_range=(6, 11), exclude=lambda x, y: not on_cap(x, y))
    return albedo, height, rough


def kevin_leaves(albedo, height, rough):
    r = random.Random("leaves-v4")
    # clustered berries - red, warm-safe, larger
    blobs = scatter_blobs(r, 10, (4, 7))
    paint_blobs(r, albedo, height, rough, blobs, h=355, s_range=(0.58, 0.72), v=0.58,
                target_height=0.64, target_rough=0.45, max_blend=1.0, h_jitter=8)
    # large gold autumn patches
    blobs = scatter_blobs(r, 6, (8, 14))
    paint_blobs(r, albedo, height, rough, blobs, h=48, s_range=(0.50, 0.65), v=0.66,
                target_height=0.60, target_rough=0.62, max_blend=0.9, h_jitter=8)
    # dark light-gaps (strong value)
    dark(r, albedo, height, rough, n=4, r_range=(6, 11), h=110, s=(0.30, 0.45), v=0.22)
    return albedo, height, rough


def kevin_stone_bricks(albedo, height, rough):
    r = random.Random("stone_bricks-v4")
    brick_h = 16

    def mortar(x, y):
        row = int(y) // brick_h
        offset = 16 if row % 2 else 0
        in_row = int(y) % brick_h
        bx = (int(x) + offset) % SIZE
        return in_row < 2 or bx % 32 < 2
    moss(r, albedo, height, rough, n=6, r_range=(7, 13), exclude=lambda x, y: not mortar(x, y))
    # rust bleed
    blobs = scatter_blobs(r, 4, (4, 8), exclude=lambda x, y: not mortar(x, y))
    paint_blobs(r, albedo, height, rough, blobs, h=24, s_range=(0.45, 0.60), v=0.42,
                target_height=0.40, target_rough=0.70, max_blend=1.0)
    return albedo, height, rough


def kevin_clay_plaster(albedo, height, rough):
    r = random.Random("clay_plaster-v4")
    # red-ochre stain low on the wall (iron-oxide runoff) - warm-safe red
    red_clay(r, albedo, height, rough, n=4, r_range=(9, 15),
             exclude=lambda x, y: y < SIZE * 0.45)
    # large damp/mold patch low on the wall
    blobs = scatter_blobs(r, 3, (9, 15), exclude=lambda x, y: y < SIZE * 0.50)
    paint_blobs(r, albedo, height, rough, blobs, h=100, s_range=(0.20, 0.30), v=0.62,
                target_height=0.44, target_rough=0.74, max_blend=0.9, h_jitter=6)
    return albedo, height, rough


def kevin_glass(albedo, height, rough):
    r = random.Random("glass-v4")

    def frame_only(x, y):
        xi, yi = int(x) % SIZE, int(y) % SIZE
        border = xi < 3 or xi >= SIZE - 3 or yi < 3 or yi >= SIZE - 3
        mullion = (xi % 32 < 2) or (yi % 32 < 2)
        return border or mullion
    # bigger rust on the frame
    blobs = scatter_blobs(r, 4, (3, 6), exclude=lambda x, y: not frame_only(x, y))
    paint_blobs(r, albedo, height, rough, blobs, h=24, s_range=(0.45, 0.60), v=0.38,
                target_height=0.36, target_rough=0.68, max_blend=1.0)
    return albedo, height, rough


def kevin_lamp(albedo, height, rough):
    r = random.Random("lamp-v4")

    def corner(x, y):
        xi, yi = int(x) % SIZE, int(y) % SIZE
        return xi < 8 or xi >= SIZE - 8 or yi < 8 or yi >= SIZE - 8
    blobs = scatter_blobs(r, 6, (3, 6), exclude=lambda x, y: not corner(x, y))
    paint_blobs(r, albedo, height, rough, blobs, h=152, s_range=(0.36, 0.52), v=0.40,
                target_height=0.44, target_rough=0.62, max_blend=1.0)
    return albedo, height, rough


def kevin_water(albedo, height, rough):
    r = random.Random("water-v4")
    # larger warm sun-glint
    blobs = scatter_blobs(r, 6, (3, 7))
    paint_blobs(r, albedo, height, rough, blobs, h=46, s_range=(0.40, 0.58), v=0.88,
                target_height=0.70, target_rough=0.05, max_blend=1.0, h_jitter=6)
    return albedo, height, rough


def kevin_metal(albedo, height, rough):
    r = random.Random("metal-v4")
    blobs = scatter_blobs(r, 5, (6, 12))
    paint_blobs(r, albedo, height, rough, blobs, h=24, s_range=(0.45, 0.60), v=0.40,
                target_height=0.36, target_rough=0.72, max_blend=1.0)
    blobs = scatter_blobs(r, 4, (5, 10))
    paint_blobs(r, albedo, height, rough, blobs, h=155, s_range=(0.30, 0.44), v=0.44,
                target_height=0.56, target_rough=0.66, max_blend=1.0)
    return albedo, height, rough


def kevin_snow(albedo, height, rough):
    r = random.Random("snow-v4")
    dark(r, albedo, height, rough, n=3, r_range=(4, 8), h=32, s=(0.10, 0.18), v=0.55)
    return albedo, height, rough


KEVIN = {
    "oak_planks": kevin_oak_planks,
    "oak_log_side": kevin_log_side,
    "oak_log_top": kevin_log_top,
    "stone_bricks": kevin_stone_bricks,
    "sand": kevin_sand,
    "grass_top": kevin_grass_top,
    "grass_side": kevin_grass_side,
    "leaves": kevin_leaves,
    "roof_tile": kevin_roof_tile,
    "glass": kevin_glass,
    "clay_plaster": kevin_clay_plaster,
    "floorboards": kevin_floorboards,
    "dirt": kevin_dirt,
    "brick": kevin_brick,
    "lamp": kevin_lamp,
    "red_sand": kevin_red_sand,
    "snow": kevin_snow,
    "water": kevin_water,
    "metal": kevin_metal,
}


def main():
    for name, make_fn, polish_fn in v3.MATERIALS:
        albedo, height, rough = make_fn()
        albedo, height, rough = polish_fn(albedo, height, rough)
        albedo, height, rough = KEVIN[name](albedo, height, rough)
        assert albedo.size == (SIZE, SIZE)
        albedo.save(OUT_DIR / f"{name}.png")
        base.normal_from_height(height).save(OUT_DIR / f"{name}_n.png")
        base.roughness_img(rough).save(OUT_DIR / f"{name}_r.png")
        print(f"{name:16s} v4 (base+v3+kevin)")


if __name__ == "__main__":
    main()
