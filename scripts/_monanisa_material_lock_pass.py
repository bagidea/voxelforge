"""
Monanisa / art lane -- 2026-08-20: "still diseased with paint_blobs" pass.

CEO looked at docs/full-block-set-realism-after-2026-08-20.png (the live
11-tile contact sheet) and found the SAME bug on 5 tiles the earlier
star-splat fix and realism pass both signed off on as OK, plus 2 tiles
("fixed") that still show it:

  oak_planks / oak_log_side / oak_log_top / brick / stone_bricks -- floating
  green (and on stone_bricks, brownish-red) disks sitting in the middle of
  the material, unrelated to any real structure.
  dirt / sand -- still-live magenta/pink + bright green on dirt, and
  turquoise + neon yellow on sand.

ROOT CAUSE (found by reading every layer of the actual pipeline that wrote
today's live files, not by re-guessing): `scatter_blobs(..., exclude=fn)`
only gates the BLOB CENTER. `paint_blobs()` then paints the full
`r_range` radius around that center regardless of exclude. Every "restricted
to mortar/seam" accent in `_pixel_blocks_gen64_v4_kevin.py` (kevin_oak_planks
has NO exclude at all; kevin_log_side has NO exclude; kevin_stone_bricks and
_monanisa_brick_regrain.py's moss call DO restrict the center to the ~2px
mortar joint, but use radius 5-16px) therefore still paints deep into the
brick/stone/plank/log FACE -- a "mortar accent" whose blob is 4-8x wider
than the mortar it's supposedly confined to. Confirmed by reading
kevin_oak_planks (moss n=4 r=8-14, bluegrey n=5 r=5-10, both with zero
exclude) and kevin_log_side (moss n=6 r=9-16, zero exclude) directly against
the screenshot -- exact match, several free-floating disks per tile.

dirt/sand: `_monanisa_realism_pass.py`'s make_dirt_v2/make_sand_v2 shrank
v4's radius but kept v4's off-material HUES verbatim: v4.red_clay is h=352
(a warm pink/magenta, not a soil colour) and v4.moss is h=115 (bright
saturated green) -- small or not, those hues don't exist in dirt. Sand still
carries a teal accent from TWO sources at once: v3.polish_sand's own
"seaglass" (h=172, never removed) plus make_sand_v2's own new h=174 blob
layered on top of it -- shrinking the second one did nothing because the
first one was never touched.

FIX, one technique, applied per material:
  1. Never call paint_blobs() with a radius that can outgrow the geometric
     feature (mortar joint / plank seam / ring band) it's meant to respect.
     For those, use `masked_speckle()` below instead -- it paints PER-TEXEL,
     gated by the same boolean mask on every pixel, so it is structurally
     unable to bleed past the mask (there is no radius to overshoot with).
  2. Only paint_blobs() for accents that are genuinely round material
     features at real scale (a wood knot, a mineral fleck) -- small radius,
     material-family hue only.
  3. Every accent hue is pulled from the material's own family:
     wood = browns only (no green/blue), brick = brick/mortar warm tones +
     one thin olive film strictly in the groove, stone_bricks = its own
     grey-blue hue for stains + a thin olive film strictly in the groove,
     dirt = its own H=27 brown family (no pink, no green), sand = its own
     H=30-42 warm neutral family (no teal, gold desaturated to "shell"
     not "neon").

Per the tile-zoom-check-procedural-texture skill: no new sine-sum, no
polar-rose paint_blobs formula (already fixed upstream in
_pixel_blocks_gen64_v3_palette.paint_blobs, reused as-is here, untouched).
No .rs touched. Writes to SCRATCH first; promote_live() is a separate,
explicit step.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402
import _pixel_blocks_gen64_v3_palette as v3  # noqa: E402
import _pixel_blocks_gen64_v4_kevin as v4  # noqa: E402
import _monanisa_brick_regrain as brickmod  # noqa: E402
import _monanisa_realism_pass as realism  # noqa: E402

SIZE = base.SIZE
jitter = base.jitter
emit = base.emit
paint_accent = v3.paint_accent
scatter_blobs = v3.scatter_blobs
paint_blobs = v3.paint_blobs
build_cell_grid = realism.build_cell_grid

SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_material_lock"
SCRATCH.mkdir(parents=True, exist_ok=True)
_LIVE_DIR = (base.ROOT / "assets" / "textures" / "blocks").resolve()
if SCRATCH.resolve() == _LIVE_DIR or _LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")


def save_scratch(img, name, suffix=""):
    p = SCRATCH / f"{name}{suffix}.png"
    img.save(p)
    return p


# --------------------------------------------------------------- primitives
def masked_speckle(rng, albedo, height, rough, mask_fn, h, s_range, v,
                    target_height, target_rough, density=0.4,
                    blend_range=(0.35, 0.75), h_jitter=4):
    """Paint per-texel, gated by mask_fn(x, y) -> bool on EVERY pixel. There
    is no radius to overshoot: coverage can never extend past the mask,
    unlike scatter_blobs(exclude=...) + paint_blobs() which only gates the
    blob CENTER and then paints the full radius regardless."""
    for y in range(SIZE):
        for x in range(SIZE):
            if not mask_fn(x, y):
                continue
            if rng.random() >= density:
                continue
            hh = h + jitter(rng, 0, h_jitter)
            ss = s_range[0] + rng.random() * (s_range[1] - s_range[0])
            blend = blend_range[0] + rng.random() * (blend_range[1] - blend_range[0])
            paint_accent(albedo, height, rough, x, y, hh, ss, v, target_height, target_rough, blend)


def root_threads(rng, albedo, height, rough, n, steps_range, h, s_range, v,
                  target_height, target_rough, blend_range=(0.45, 0.75)):
    """Thin 1px random-walk threads (roots/twigs), never a filled area."""
    for _ in range(n):
        x = rng.uniform(0, SIZE)
        y = rng.uniform(0, SIZE)
        ang = rng.uniform(0, math.tau)
        steps = rng.randint(*steps_range)
        for _s in range(steps):
            ang += jitter(rng, 0, 0.5)
            x = (x + math.cos(ang)) % SIZE
            y = (y + math.sin(ang)) % SIZE
            if rng.random() < 0.7:
                hh = h + jitter(rng, 0, 4)
                ss = s_range[0] + rng.random() * (s_range[1] - s_range[0])
                blend = blend_range[0] + rng.random() * (blend_range[1] - blend_range[0])
                paint_accent(albedo, height, rough, int(x), int(y), hh, ss, v,
                             target_height, target_rough, blend)


# ----------------------------------------------------------------- oak_planks
def make_oak_planks_v3():
    albedo, height, rough = base.make_oak_planks()
    v3.polish_oak_planks(albedo, height, rough)  # tiny grey nail heads only, kept

    r = random.Random("oak_planks-lock")
    plank_w = 16

    # one dark walnut knot, brown family only (was: unconstrained green moss
    # r=8-14 + blue-grey r=5-10 floating anywhere on the grain)
    for plank in (1, 3):
        if r.random() < 0.7:
            cx = plank * plank_w + r.uniform(4, plank_w - 5)
            cy = r.uniform(8, SIZE - 8)
            ring = scatter_blobs(r, 1, (2.2, 3.4))
            if ring:
                cx0, cy0, rr0, w0, f0, p0 = ring[0]
                paint_blobs(r, albedo, height, rough, [(cx, cy, rr0, w0, f0, p0)],
                            h=24, s_range=(0.30, 0.42), v=0.24,
                            target_height=0.28, target_rough=0.62, max_blend=0.9, h_jitter=4)

    # faint moisture stain along ONE existing seam column -- masked to the
    # seam pixels the base pass already darkens, brown family, never spills
    # onto the board face
    stain_col = r.choice([0, 1, 2, 3]) * plank_w + plank_w - 1

    def on_stain_seam(x, y):
        return int(x) == stain_col and y > SIZE * 0.4

    masked_speckle(r, albedo, height, rough, on_stain_seam,
                    h=22, s_range=(0.20, 0.32), v=0.20,
                    target_height=0.26, target_rough=0.70, density=0.5, h_jitter=3)
    return albedo, height, rough


# -------------------------------------------------------------- oak_log_side
def make_log_side_v3():
    albedo, height, rough = base.make_log_side()
    # NOTE: intentionally skip v3.polish_log_side (h=96 green lichen) -- wood
    # accents must stay in the brown family.

    r = random.Random("oak_log_side")  # same seed base.make_log_side used,
    cracks = sorted(r.sample(range(2, SIZE - 2), 12))  # reproduces its crack columns

    r2 = random.Random("oak_log_side-lock")

    def near_crack(x, y):
        return min(abs(int(x) - c) for c in cracks) <= 1

    # deepen the cracks with dark bark accretion, brown/near-black only
    masked_speckle(r2, albedo, height, rough, near_crack,
                    h=18, s_range=(0.10, 0.20), v=0.12,
                    target_height=0.20, target_rough=0.90, density=0.55, h_jitter=3)

    # one small amber sap stain at a crack, warm-safe, tight radius
    c = r2.choice(cracks)
    y0 = r2.uniform(10, SIZE - 10)
    blobs = scatter_blobs(r2, 1, (2.0, 3.0))
    if blobs:
        cx0, cy0, rr0, w0, f0, p0 = blobs[0]
        paint_blobs(r2, albedo, height, rough, [(c, y0, rr0, w0, f0, p0)],
                    h=32, s_range=(0.45, 0.58), v=0.38,
                    target_height=0.50, target_rough=0.55, max_blend=0.75, h_jitter=4)
    return albedo, height, rough


# --------------------------------------------------------------- oak_log_top
def make_log_top_v3():
    albedo, height, rough = base.make_log_top()
    # skip polish_log_top entirely (h=200 blue-grey spalt + h=98 green ring
    # dots) -- replaced with small brown knots anchored to the ring pattern.
    r = random.Random("oak_log_top-lock")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2

    def ring_band(x, y):
        d = math.hypot(x - cx, y - cy)
        return 10 < d < 26

    blobs = scatter_blobs(r, 2, (2.0, 3.2), exclude=lambda x, y: not ring_band(x, y))
    paint_blobs(r, albedo, height, rough, blobs, h=22, s_range=(0.32, 0.44), v=0.22,
                target_height=0.26, target_rough=0.75, max_blend=0.85, h_jitter=4)
    return albedo, height, rough


# --------------------------------------------------------------------- brick
def make_brick_v3():
    # the ONLY thing wrong with this tile is brickmod.make_brick()'s final
    # v4.moss() call (green, r=5-9, mortar-center-only so it still bleeds
    # into the face). Its base per-texel loop + clinker/overfired rows are
    # locked and good, so reproduce those here without the moss tail.
    albedo, height, rough = _rebuild_brick_base()

    r = random.Random("brick-lock")
    brick_h = 16

    def mortar(x, y):
        row = int(y) // brick_h
        offset = 16 if row % 2 else 0
        in_row = int(y) % brick_h
        bx = (int(x) + offset) % SIZE
        return in_row < 2 or bx % 32 < 2

    # pale efflorescence (mineral salt bloom), warm-neutral, strictly in the
    # groove -- replaces the green moss blob
    masked_speckle(r, albedo, height, rough, mortar,
                    h=36, s_range=(0.06, 0.14), v=0.72,
                    target_height=0.55, target_rough=0.60, density=0.30, h_jitter=6)
    # soot smudge, dark warm-neutral, sparser
    masked_speckle(r, albedo, height, rough, mortar,
                    h=26, s_range=(0.08, 0.16), v=0.18,
                    target_height=0.30, target_rough=0.75, density=0.14, h_jitter=4)
    return albedo, height, rough


def _rebuild_brick_base():
    """Same per-texel loop + clinker/overfired rows as
    _monanisa_brick_regrain.make_brick, minus its trailing v4.moss() call."""
    albedo, height, rough = base.new_state()
    fine_rng = random.Random("brick-monanisa-fine")
    BRICK_H = brickmod.BRICK_H
    BRICK_H0, BRICK_S0, BRICK_V0 = brickmod.BRICK_H0, brickmod.BRICK_S0, brickmod.BRICK_V0
    MORTAR_H0, MORTAR_S0, MORTAR_V0 = brickmod.MORTAR_H0, brickmod.MORTAR_S0, brickmod.MORTAR_V0

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
                v_ref = BRICK_V0 + base.jitter(brng, 0, 0.11)
                h = BRICK_H0 + base.jitter(brng, 0, 6)
                s = BRICK_S0 + base.jitter(brng, 0, 0.08)
                tau = math.tau
                grain = (math.sin(tau * (17 * x + 13 * y) / SIZE + row * 2.1) * 0.035
                         + math.sin(tau * (9 * x - 21 * y) / SIZE + row) * 0.02)
                v = v_ref + grain + base.jitter(fine_rng, 0, 0.025)
                rr = 0.60
            base.emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)

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
    return albedo, height, rough


# --------------------------------------------------------------- stone_bricks
def make_stone_bricks_v3():
    albedo, height, rough = base.make_stone_bricks()
    # skip polish_stone_bricks + kevin_stone_bricks entirely -- both used
    # scatter_blobs(exclude=mortar-center) + paint_blobs at r up to 13px,
    # which is why the groove accent ballooned into a disk mid-block.

    r = random.Random("stone_bricks-lock")
    brick_h = 16

    def mortar(x, y):
        row = int(y) // brick_h
        offset = 16 if row % 2 else 0
        in_row = int(y) % brick_h
        bx = (int(x) + offset) % SIZE
        return in_row < 2 or bx % 32 < 2

    # thin olive moss FILM, strictly in the groove pixels -- this is the
    # "if you use moss it must be a film in the mortar" instruction
    masked_speckle(r, albedo, height, rough, mortar,
                    h=96, s_range=(0.26, 0.38), v=0.36,
                    target_height=0.58, target_rough=0.82, density=0.45, h_jitter=5)
    # sparse rust bleed at the groove, small and restrained
    masked_speckle(r, albedo, height, rough, mortar,
                    h=22, s_range=(0.30, 0.42), v=0.32,
                    target_height=0.38, target_rough=0.70, density=0.10, h_jitter=4)
    # dark grey weather stains / hairline cracks on the STONE FACE, same hue
    # family as the stone itself (H~212), just darker -- never a foreign hue
    masked_speckle(r, albedo, height, rough, lambda x, y: not mortar(x, y),
                    h=212, s_range=(0.06, 0.12), v=0.28,
                    target_height=0.32, target_rough=0.78, density=0.045, h_jitter=8)
    return albedo, height, rough


# ------------------------------------------------------------------- dirt
def make_dirt_v3():
    albedo, height, rough = base.make_dirt()
    r = random.Random("dirt-lock")

    # soft richer/paler soil clumps, SAME hue family as dirt (H~27) --
    # value-only variation via a cellular field, so it reads as "patch of
    # soil" not "patch of a different material" (was: h=352 pink + h=115
    # green from v4.red_clay/moss)
    N_CLUMPS = 10
    tone_rng = random.Random("dirt-lock-clump-tone")
    clump_dv = [jitter(tone_rng, 0, 0.09) for _ in range(N_CLUMPS)]
    idx, _ = build_cell_grid("dirt-lock-cells", N_CLUMPS, aniso_range=(1.6, 2.6))
    H, S, V = 27, 0.42, 0.36
    for y in range(SIZE):
        for x in range(SIZE):
            i = idx[y][x]
            dv = clump_dv[i]
            if abs(dv) < 0.02:
                continue
            v = max(0.05, min(0.95, V + dv))
            s = max(0.05, S - dv * 0.3)
            paint_accent(albedo, height, rough, x, y, H, s, v,
                         0.5 + dv * 1.8, 0.90, 0.35)

    # dark reddish-brown root threads -- thin, never a filled area
    root_threads(r, albedo, height, rough, n=4, steps_range=(6, 12),
                 h=14, s_range=(0.20, 0.30), v=0.14,
                 target_height=0.22, target_rough=0.85)

    # keep the warm mineral flecks that were already fine (pale pebble,
    # dark crumb) at a light touch -- both already in the dirt hue family
    v4.pale(r, albedo, height, rough, n=2, r_range=(1.5, 3), h=38, s=(0.06, 0.12), v=0.55)
    v4.dark(r, albedo, height, rough, n=2, r_range=(1.5, 3), h=24, s=(0.10, 0.18), v=0.16)
    return albedo, height, rough


# ------------------------------------------------------------------- sand
def make_sand_v3():
    albedo, height, rough = base.make_sand()
    r = random.Random("sand-lock")

    # pale shell fleck -- off-white, tiny, already the right hue family
    shells = scatter_blobs(r, 6, (0.9, 1.5))
    paint_blobs(r, albedo, height, rough, shells, h=24, s_range=(0.05, 0.12), v=0.90,
                target_height=0.60, target_rough=0.55, max_blend=0.8, h_jitter=8)

    # grey-brown pebble fleck -- dark, small, warm-neutral (was v4.dark, kept
    # but tuned slightly warmer/smaller)
    pebbles = scatter_blobs(r, 6, (0.7, 1.3))
    paint_blobs(r, albedo, height, rough, pebbles, h=30, s_range=(0.08, 0.16), v=0.30,
                target_height=0.36, target_rough=0.68, max_blend=0.85, h_jitter=6)

    # sun-bleached mineral fleck -- was v4.gold at s=0.60-0.75/v=0.82 (reads
    # as neon yellow dot); pulled back into the sand's own warm hue, much
    # lower chroma so it reads as a mineral glint not a foreign colour
    glints = scatter_blobs(r, 3, (1.5, 2.5))
    paint_blobs(r, albedo, height, rough, glints, h=40, s_range=(0.22, 0.32), v=0.72,
                target_height=0.50, target_rough=0.40, max_blend=0.7, h_jitter=5)

    # NOTE: intentionally no h=172/174 teal accent anywhere in this pass --
    # that hue does not exist in sand; it was double-sourced from
    # v3.polish_sand's "seaglass" AND the previous realism-pass's own extra
    # blob, both removed by not calling polish_sand here at all.
    return albedo, height, rough


TARGETS = [
    ("oak_planks", make_oak_planks_v3),
    ("oak_log_side", make_log_side_v3),
    ("oak_log_top", make_log_top_v3),
    ("brick", make_brick_v3),
    ("stone_bricks", make_stone_bricks_v3),
    ("dirt", make_dirt_v3),
    ("sand", make_sand_v3),
]


def main():
    names = sys.argv[1:] or [n for n, _ in TARGETS]
    for name, fn in TARGETS:
        if name not in names:
            continue
        albedo, height, rough = fn()
        assert albedo.size == (SIZE, SIZE)
        save_scratch(albedo, name)
        save_scratch(base.normal_from_height(height), name, "_n")
        save_scratch(base.roughness_img(rough), name, "_r")
        print(f"wrote {name} (+ _n/_r) to {SCRATCH}")


if __name__ == "__main__":
    main()
