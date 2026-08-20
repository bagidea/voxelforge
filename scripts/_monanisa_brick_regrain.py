"""
Monanisa / art lane — 2026-08-20: fix brick.png's flat/uniform look.

ROOT CAUSE (found by audit, not guessed): brick.png's live pixels come from
`_kevin_palette_push.py` (commit 5ebe745), a one-off pass that repainted 5
OUTDOOR-dominant materials (grass_top, grass_side, leaves, brick, roof_tile)
with flat 2-tone fills + a single low-frequency sine, purely to survive hue
collapse under the beach_dusk sunset key light. That pass bypassed the normal
base -> v3 -> v4 pipeline every OTHER interior tile still carries (per-brick
random jitter, fine grain, clinker/overfired variation, moss weathering) --
so brick.png alone regressed to near-monochrome bands while its siblings
(oak_planks, oak_log_*, floorboards, stone_bricks, clay_plaster) kept their
per-tile detail. Confirmed by `_monanisa_texture_audit.py`: brick had the
lowest hue_bins_used (3/36 vs 7-15 for the rest) and the narrowest value
floor (v_min 0.604, i.e. never gets darker than a mid-tone -- no shadowed
mortar recess, no soot/weathering).

FIX: rebuild brick using the SAME locked base/mortar RGB and running-bond
layout 5ebe745 set (so the outdoor hue-survival property it was written for
does not regress), but replace the flat single-sine modulation with the
per-brick individual jitter + fine surface grain the base pipeline gives
every other tile, plus the clinker/overfired per-row variation and moss
weathering `kevin_brick()` in _pixel_blocks_gen64_v4_kevin.py already wrote
for exactly this tile (that pass changed brick_n/_r in ae81561 but never
touched brick.png's albedo, because 5ebe745 overwrote it afterward with a
different generator -- so this reuses the v4 design and finally lands it on
the base 5ebe745 chose).

Height/normal/roughness are derived from the SAME per-texel pass as albedo
(project convention, see _pixel_blocks_gen64.py header) so the PBR triplet
stays correlated.

Reuses base.emit/hsv/jitter + v3/v4's scatter_blobs/paint_blobs/moss/
paint_accent helpers. No Rust, no rebuild, no palette-family change.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402
import _pixel_blocks_gen64_v3_palette as v3  # noqa: E402
import _pixel_blocks_gen64_v4_kevin as v4  # noqa: E402

SIZE = base.SIZE
BRICK_H = 16  # 4 courses of 16px = 64px, matches the locked 5ebe745 layout

# Locked base colours from _kevin_palette_push.py (5ebe745) -- do not drift,
# this is the hue that survives the warm sunset key light.
BRICK_RGB = (182, 66, 42)
MORTAR_RGB = (208, 148, 118)


def rgb_to_hsv01(rgb):
    import colorsys
    r, g, b = (c / 255 for c in rgb)
    return colorsys.rgb_to_hsv(r, g, b)


BRICK_H0, BRICK_S0, BRICK_V0 = rgb_to_hsv01(BRICK_RGB)
MORTAR_H0, MORTAR_S0, MORTAR_V0 = rgb_to_hsv01(MORTAR_RGB)
BRICK_H0 *= 360
MORTAR_H0 *= 360


def make_brick():
    albedo, height, rough = base.new_state()
    fine_rng = random.Random("brick-monanisa-fine")

    for y in range(SIZE):
        row = y // BRICK_H
        offset = 16 if row % 2 else 0
        in_row = y % BRICK_H
        for x in range(SIZE):
            bx = (x + offset) % SIZE
            mortar = in_row < 2 or bx % 32 < 2
            brick_id = f"brick-{row}-{(x + offset) // 32}"
            brng = random.Random(brick_id)

            if mortar:
                v_ref = MORTAR_V0
                h = MORTAR_H0 + base.jitter(brng, 0, 4)
                s = MORTAR_S0 + base.jitter(brng, 0, 0.04)
                v = v_ref + base.jitter(brng, 0, 0.05) + base.jitter(fine_rng, 0, 0.02)
                rr = 0.86
            else:
                # per-brick individual tone (was a single flat fill before --
                # this is the #1 reason it read as one uniform colour)
                v_ref = BRICK_V0 + base.jitter(brng, 0, 0.11)
                h = BRICK_H0 + base.jitter(brng, 0, 6)
                s = BRICK_S0 + base.jitter(brng, 0, 0.08)
                # fine fired-clay surface grain: two integer-cycle plane
                # waves (seamless over the 64px tile period, no fractional
                # per-pixel frequency) at incommensurate ratios so it doesn't
                # read as a single visible ripple
                tau = math.tau
                grain = (math.sin(tau * (17 * x + 13 * y) / SIZE + row * 2.1) * 0.035
                         + math.sin(tau * (9 * x - 21 * y) / SIZE + row) * 0.02)
                v = v_ref + grain + base.jitter(fine_rng, 0, 0.025)
                rr = 0.60

            base.emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)

    # per-row clinker (overfired blue-black) / overfired-red whole bricks --
    # same vocabulary _pixel_blocks_gen64_v4_kevin.py already designed for
    # this tile (kevin_brick), never landed because 5ebe745 overwrote the
    # albedo it would have applied to.
    r = random.Random("brick-v4-monanisa")
    for row in range(SIZE // BRICK_H):
        offset = 16 if row % 2 else 0
        roll = r.random()
        if roll < 0.14:
            for yy in range(row * BRICK_H + 2, row * BRICK_H + BRICK_H):
                for xx in range(SIZE):
                    bx = (xx + offset) % SIZE
                    if bx % 32 >= 2:
                        v3.paint_accent(albedo, height, rough, xx, yy,
                                        226, 0.10, 0.30, 0.34, 0.55, 0.78)
        elif roll < 0.30:
            for yy in range(row * BRICK_H + 2, row * BRICK_H + BRICK_H):
                for xx in range(SIZE):
                    bx = (xx + offset) % SIZE
                    if bx % 32 >= 2:
                        v3.paint_accent(albedo, height, rough, xx, yy,
                                        355, 0.62, 0.52, 0.44, 0.70, 0.68)

    def exclude_brick_face(x, y):
        # moss grows in the mortar grooves, not the fired brick face --
        # same convention kevin_brick() used
        yy = int(y) % BRICK_H
        row = int(y) // BRICK_H
        offset = 16 if row % 2 else 0
        bx = (int(x) + offset) % SIZE
        return yy >= 2 and bx % 32 >= 2

    v4.moss(r, albedo, height, rough, n=2, r_range=(5, 9), exclude=exclude_brick_face)

    return albedo, height, rough


def main():
    albedo, height, rough = make_brick()
    base.save(albedo, "brick")
    base.save(base.normal_from_height(height), "brick", "_n")
    base.save(base.roughness_img(rough), "brick", "_r")
    print("wrote brick.png, brick_n.png, brick_r.png")


if __name__ == "__main__":
    main()
