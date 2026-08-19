"""
Monanisa / art lane -- environment pack: sky, water re-pass, vegetation cross-sprites.

Three independent deliverables, all 64x64 (matching the 64px PBR block set
already in assets/textures/blocks/), all pure PIL + deterministic seeding
(re-running reproduces byte-identical output), same "author a height field,
derive normal+roughness from it" pipeline as scripts/_pixel_blocks_gen64.py.

1. SKY  (assets/textures/sky/) -- not part of the block atlas, a different
   asset family for whatever cloud-layer system consumes it (Poppy's lane):
     sky_gradient_sunset.png -- vertical ramp, top=zenith to bottom=horizon,
                                 stops sampled directly off
                                 docs/refs/ceo_ref_sunset_valley.jpg (indigo
                                 -> mauve -> rose -> coral -> orange -> gold).
     sky_clouds.png          -- seamless-tileable cumulus texture, RGBA
                                 (alpha = cloud coverage). Built from fbm
                                 value noise on wrapping integer-resolution
                                 grids (random per-vertex, not a smooth
                                 sine sum -- a first pass using a handful of
                                 integer-frequency sine terms tiled cleanly
                                 but interfered into a visible diagonal
                                 lattice once zoomed/tiled 2x2; noise octaves
                                 don't share that failure mode) -- soft
                                 painterly clouds, warm-lit crown / cool-
                                 shadow base.
     sun_disk.png             -- radial glow sprite, RGBA, warm white core to
                                 transparent haze, plus a faint horizon glint.

2. WATER (assets/textures/blocks/water*.png) -- the existing water.png reads
   as flat-blue speckle noise up close (confirmed by a 512x512 nearest-
   neighbour zoom -- no visible wave *shape*, and its wave terms use
   non-integer frequencies so the tile does not even wrap perfectly). Not
   good enough for the new 64px set. Re-authored with the same HSV target
   (205 deg, S0.55, V0.55 -- unchanged, so nothing downstream that reads the
   old hue moves) but with real directional ripple bands (two low-freq
   integer-cycle swell terms) whose phase is domain-warped by a tileable
   noise field, plus a separate higher-octave fbm "chop" layer on top --
   first pass used the swell terms bare, which (like the clouds) interfered
   into a visible diagonal argyle at 2x2 zoom; the noise warp bends the
   crest lines organically instead of leaving them dead straight, and stays
   exactly period-64 because the warp/chop fields are themselves tileable
   noise. Both the albedo AND the height field (hence the normal map) show
   actual wave structure. Roughness is now height-coupled via `emit()` like
   every other material (crests glossier, troughs rougher) instead of a
   flat override, plus sparse near-mirror sun-glint accents.

3. VEGETATION (assets/textures/blocks/vegetation/) -- 7 alpha-cutout cross-
   sprites (flowers/foliage/grass/tree leaves) for the cross-block render
   mode the engine doesn't have yet. Registered in atlas.json's `tiles` list
   only (no `kinds`) -- same "ready to wire, currently inert" precedent as
   `water`/`metal` in the first PBR pass: the loader validates + packs them,
   nothing references them by kind, so today's render is unaffected. Built
   from simple signed-distance primitives (capsule stems/blades/twigs, polar
   lobe functions for petals, unions of circles for foliage clumps)
   compositely painted (proper Porter-Duff "over") onto a transparent 64x64
   canvas, same emit-derived normal/roughness pipeline as everything else.

No Rust changed. No maps/ changed. atlas.json only gains new `tiles` rows
(see bottom of this file / the hand-edit that follows running it).
"""

import colorsys
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BLOCKS_DIR = ROOT / "assets" / "textures" / "blocks"
VEG_DIR = BLOCKS_DIR / "vegetation"
SKY_DIR = ROOT / "assets" / "textures" / "sky"
REF_IMG = ROOT / "docs" / "refs" / "ceo_ref_sunset_valley.jpg"
VEG_DIR.mkdir(parents=True, exist_ok=True)
SKY_DIR.mkdir(parents=True, exist_ok=True)

SIZE = 64

HEIGHT_GAIN = 1.8
ROUGH_GAIN = 0.55


# --------------------------------------------------------------------- math

def clamp01(v):
    return max(0.0, min(1.0, v))


def lerp(a, b, t):
    return a + (b - a) * t


def smoothstep(e0, e1, x):
    if e0 == e1:
        return 1.0 if x >= e1 else 0.0
    t = clamp01((x - e0) / (e1 - e0))
    return t * t * (3 - 2 * t)


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb((h % 360) / 360.0, clamp01(s), clamp01(v))
    return (round(r * 255), round(g * 255), round(b * 255), 255)


def hsv01(h, s, v):
    return colorsys.hsv_to_rgb((h % 360) / 360.0, clamp01(s), clamp01(v))


def hex_rgb01(hx):
    hx = hx.lstrip("#")
    return (int(hx[0:2], 16) / 255.0, int(hx[2:4], 16) / 255.0, int(hx[4:6], 16) / 255.0)


def lerp3(a, b, t):
    return tuple(lerp(a[i], b[i], t) for i in range(3))


def jitter(rng, base, amt):
    return base + (rng.random() * 2 - 1) * amt


# ------------------------------------------------- tileable value noise (fbm)
#
# A sum of a handful of INTEGER-frequency sine terms tiles perfectly, but a
# small set of plane waves is exactly what a Fourier series is -- summing
# few of them reproduces their own interference (a regular diagonal
# argyle/lattice/moire), which is the opposite of organic. Real per-pixel
# noise would fix that but breaks the wraparound seam. The standard way to
# get both: value noise on a small integer-resolution grid, where GRID
# WRAPS (vertex `grid` == vertex `0`) so any single octave is exactly
# period-1 over the unit square, and every vertex is an *independent random
# value* rather than a smooth analytic function of position -- so summing
# octaves doesn't recreate a lattice the way summing sinusoids does.

def tileable_value_grid(rng, grid):
    return [[rng.uniform(-1.0, 1.0) for _ in range(grid)] for _ in range(grid)]


def sample_tileable(grid_vals, grid, fx, fy):
    gx, gy = fx * grid, fy * grid
    gx0f, gy0f = math.floor(gx), math.floor(gy)
    gx0, gy0 = int(gx0f) % grid, int(gy0f) % grid
    gx1, gy1 = (gx0 + 1) % grid, (gy0 + 1) % grid
    tx, ty = gx - gx0f, gy - gy0f
    sx = tx * tx * (3 - 2 * tx)
    sy = ty * ty * (3 - 2 * ty)
    v00, v10 = grid_vals[gy0][gx0], grid_vals[gy0][gx1]
    v01, v11 = grid_vals[gy1][gx0], grid_vals[gy1][gx1]
    top = v00 + (v10 - v00) * sx
    bot = v01 + (v11 - v01) * sx
    return top + (bot - top) * sy


def build_fbm_octaves(rng, spec):
    """spec: list of (grid, amplitude). Returns [(grid, amp, grid_vals), ...]."""
    return [(grid, amp, tileable_value_grid(rng, grid)) for grid, amp in spec]


def sample_fbm(octaves, fx, fy):
    return sum(amp * sample_tileable(vals, grid, fx, fy) for grid, amp, vals in octaves)


def sample_fbm_warped(base_octaves, warp_x_octaves, warp_y_octaves, fx, fy, warp_amt):
    """Domain-warp: displace the sample point by a second (independent) noise
    field before reading the base fbm. A low-vertex-count value-noise grid
    interpolates into a very clean, near-symmetric diamond/saddle shape on
    its own (confirmed by a 2x2 tiled zoom -- that read as an argyle lattice
    just like the sine version it replaced). Warping the lookup coordinate
    with its own periodic noise bends those symmetric interpolation lines
    into irregular curls, which is what actually reads as organic. Both warp
    and base stay periodic over [0,1)x[0,1), so the result still tiles
    exactly."""
    wx = sample_fbm(warp_x_octaves, fx, fy) * warp_amt
    wy = sample_fbm(warp_y_octaves, fx, fy) * warp_amt
    return sample_fbm(base_octaves, (fx + wx) % 1.0, (fy + wy) % 1.0)


# ------------------------------------------------------ shared block-style state

def new_state():
    albedo = Image.new("RGBA", (SIZE, SIZE))
    height = [[0.5] * SIZE for _ in range(SIZE)]
    rough = [[0.5] * SIZE for _ in range(SIZE)]
    return albedo, height, rough


def emit(albedo, height, rough, x, y, h, s, v, v_ref, base_rough, rough_override=None):
    dv = v - v_ref
    albedo.putpixel((x, y), hsv(h, s, v))
    height[y][x] = clamp01(0.5 + dv * HEIGHT_GAIN)
    rough[y][x] = rough_override if rough_override is not None else clamp01(base_rough - dv * ROUGH_GAIN)


def normal_from_height(height, strength=2.4, wrap=True):
    img = Image.new("RGBA", (SIZE, SIZE))
    for y in range(SIZE):
        for x in range(SIZE):
            if wrap:
                hl = height[y][(x - 1) % SIZE]
                hr = height[y][(x + 1) % SIZE]
                hu = height[(y - 1) % SIZE][x]
                hd = height[(y + 1) % SIZE][x]
            else:
                hl = height[y][max(x - 1, 0)]
                hr = height[y][min(x + 1, SIZE - 1)]
                hu = height[max(y - 1, 0)][x]
                hd = height[min(y + 1, SIZE - 1)][x]
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


def avg_hex(img):
    px = list(img.convert("RGB").getdata())
    avg = tuple(round(sum(c[i] for c in px) / len(px)) for i in range(3))
    return f"#{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}"


def save(img, out_dir, name, suffix=""):
    path = out_dir / f"{name}{suffix}.png"
    img.save(path)
    assert img.size == (SIZE, SIZE)
    return path


# =====================================================================
# 1) SKY
# =====================================================================

SKY_STOPS = [
    (0.00, "#3a2f52"),  # zenith -- deep indigo
    (0.15, "#5c3f68"),  # violet-mauve
    (0.32, "#96536f"),  # dusty rose-mauve
    (0.48, "#d06d72"),  # rose-coral
    (0.62, "#f3835f"),  # orange
    (0.76, "#ffab5e"),  # amber-gold
    (0.90, "#ffd08a"),  # warm gold
    (1.00, "#fff0c2"),  # pale gold-white, horizon glow
]


def make_sky_gradient():
    """Vertical ramp, sampled off docs/refs/ceo_ref_sunset_valley.jpg's sky
    column (see PALETTE.md addendum for the raw samples). Column-uniform so
    it reads as a clean 1D LUT a shader can sample by V; tiny per-channel
    dither breaks 8-bit banding once stretched across a sky dome."""
    img = Image.new("RGBA", (SIZE, SIZE))
    rng = random.Random("sky_gradient")
    stops = [(f, hex_rgb01(c)) for f, c in SKY_STOPS]
    for y in range(SIZE):
        f = y / (SIZE - 1)
        i = 0
        while i < len(stops) - 2 and f > stops[i + 1][0]:
            i += 1
        f0, c0 = stops[i]
        f1, c1 = stops[i + 1]
        local_t = 0.0 if f1 == f0 else clamp01((f - f0) / (f1 - f0))
        col = lerp3(c0, c1, smoothstep(0, 1, local_t))
        for x in range(SIZE):
            dr = jitter(rng, 0, 1.2 / 255)
            dg = jitter(rng, 0, 1.2 / 255)
            db = jitter(rng, 0, 1.2 / 255)
            r = round(clamp01(col[0] + dr) * 255)
            g = round(clamp01(col[1] + dg) * 255)
            b = round(clamp01(col[2] + db) * 255)
            img.putpixel((x, y), (r, g, b, 255))
    return img


# Tileable value-noise fbm, domain-warped (see sample_fbm_warped above) --
# a bare few-octave fbm on small grids still interpolates into a clean,
# near-symmetric diamond/saddle shape (confirmed by a 2x2 tiled zoom); the
# warp field bends those symmetric interpolation lines into irregular
# curls, which is what actually reads as soft cumulus instead of a
# repeating lattice.
CLOUD_OCTAVE_SPEC = [(6, 1.00), (11, 0.46), (17, 0.26), (27, 0.14)]
CLOUD_WARP_X_SPEC = [(3, 1.00)]
CLOUD_WARP_Y_SPEC = [(4, 1.00)]
CLOUD_WARP_AMOUNT = 0.10


def make_sky_clouds():
    img = Image.new("RGBA", (SIZE, SIZE))
    rng = random.Random("sky_clouds")
    octaves = build_fbm_octaves(rng, CLOUD_OCTAVE_SPEC)
    warp_x = build_fbm_octaves(random.Random("sky_clouds_warp_x"), CLOUD_WARP_X_SPEC)
    warp_y = build_fbm_octaves(random.Random("sky_clouds_warp_y"), CLOUD_WARP_Y_SPEC)
    raw = [[0.0] * SIZE for _ in range(SIZE)]
    lo, hi = 1e9, -1e9
    for y in range(SIZE):
        fy = y / SIZE
        for x in range(SIZE):
            fx = x / SIZE
            v = sample_fbm_warped(octaves, warp_x, warp_y, fx, fy, CLOUD_WARP_AMOUNT)
            raw[y][x] = v
            lo, hi = min(lo, v), max(hi, v)
    lit = hsv01(32, 0.16, 0.98)
    shadow = hsv01(228, 0.20, 0.74)
    thr, edge = 0.58, 0.22
    span = max(1e-6, hi - lo)
    for y in range(SIZE):
        for x in range(SIZE):
            nv = clamp01((raw[y][x] - lo) / span)
            a = smoothstep(thr - edge, thr + edge, nv)
            shade_t = clamp01((nv - thr) / max(1e-4, 1 - thr))
            col = lerp3(shadow, lit, shade_t)
            img.putpixel((x, y), (round(col[0] * 255), round(col[1] * 255), round(col[2] * 255), round(a * 255)))
    return img


def make_sun_disk():
    img = Image.new("RGBA", (SIZE, SIZE))
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
    hot = hsv01(46, 0.10, 1.0)
    gold = hsv01(36, 0.55, 1.0)
    edge_c = hsv01(24, 0.70, 1.0)
    max_r = 29.0
    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy)
            t = clamp01(d / max_r)
            a = clamp01(1.15 * (1 - t) ** 1.8)
            if abs(y - cy) < 1.5 and d < max_r:
                a = min(1.0, a + 0.15 * (1 - t))
            col_t = clamp01(1 - t * 1.6)
            col = lerp3(edge_c, lerp3(gold, hot, col_t), col_t)
            img.putpixel((x, y), (round(col[0] * 255), round(col[1] * 255), round(col[2] * 255), round(a * 255)))
    return img


def build_sky_contact_sheet():
    cell = 256
    pad, label_h = 10, 22
    names = ["sky_gradient_sunset (ramp)", "sky_clouds (tiled 2x2)", "sun_disk"]
    W = 3 * (cell + pad) + pad
    Hh = cell + label_h + 2 * pad
    sheet = Image.new("RGB", (W, Hh), (18, 19, 24))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 14)
    except Exception:
        font = ImageFont.load_default()

    grad = Image.open(SKY_DIR / "sky_gradient_sunset.png").convert("RGB").resize((cell, cell), Image.NEAREST)
    clouds = Image.open(SKY_DIR / "sky_clouds.png").convert("RGBA")
    tiled = Image.new("RGBA", (SIZE * 2, SIZE * 2))
    for j in range(2):
        for i in range(2):
            tiled.paste(clouds, (i * SIZE, j * SIZE))
    dusk_bg = Image.new("RGBA", tiled.size, (86, 62, 96, 255))
    cloud_preview = Image.alpha_composite(dusk_bg, tiled).convert("RGB").resize((cell, cell), Image.NEAREST)

    sun = Image.open(SKY_DIR / "sun_disk.png").convert("RGBA")
    night_bg = Image.new("RGBA", sun.size, (30, 24, 46, 255))
    sun_preview = Image.alpha_composite(night_bg, sun).convert("RGB").resize((cell, cell), Image.NEAREST)

    for i, tile in enumerate([grad, cloud_preview, sun_preview]):
        x0 = pad + i * (cell + pad)
        sheet.paste(tile, (x0, label_h + pad))
        draw.rectangle([x0, label_h + pad, x0 + cell - 1, label_h + pad + cell - 1], outline=(70, 70, 78), width=1)
        draw.text((x0, 2), names[i], fill=(230, 230, 230), font=font)

    out = SKY_DIR / "sky_contact_sheet.png"
    sheet.save(out)
    print(f"sky contact sheet -> {out}")
    return out


# =====================================================================
# 2) WATER (re-pass, lives in assets/textures/blocks/)
# =====================================================================

# Two low-frequency integer-cycle swell terms give the ripple its directional
# "wave crest" character, but two-or-three bare plane waves is exactly a
# Fourier series -- summing them alone reproduces their own interference (a
# regular diagonal argyle grid), the same failure mode as the old clouds.
# Fixed by domain-warping the swell's phase with a tileable noise field (so
# crest lines bend organically instead of running dead straight) and adding
# a separate higher-octave fbm "chop" layer on top for fine, non-periodic
# bump detail. The warp/chop fields are themselves built from
# build_fbm_octaves (random-per-vertex, wraps exactly), so the whole sum
# stays exactly period-64 -- still a true seamless tile, just no longer a
# lattice.
# Grids are deliberately non-power-of-two / non-common-multiple (5, 9, 13,
# 19 rather than 4, 8, 16, 32) -- harmonically related grid sizes re-align
# their vertices octave over octave and reproduce a checker/argyle lattice
# just like a handful of sine terms does (confirmed by a 2x2 tiled zoom: the
# first pass, on power-of-two grids with a mild warp, still showed a clear
# regular diagonal band + diamond-dot lattice). The warp is also much
# stronger here and the chop layer is now the dominant term, with the two
# directional swells reduced to a subtle underlying push -- enough to bend
# the crest lines into organic curves instead of visible straight bands.
WATER_WARP_SPEC = [(3, 1.00), (5, 0.55)]
WATER_CHOP_SPEC = [(5, 1.00), (9, 0.55), (13, 0.32), (19, 0.18)]


def water_ripple(warp_octaves, chop_octaves, fx, fy):
    warp = sample_fbm(warp_octaves, fx, fy) * 2.2
    swell_a = math.sin(2 * math.pi * (2 * fx + 3 * fy) + warp)
    swell_b = math.sin(2 * math.pi * (-3 * fx + 2 * fy) + 1.3 + warp * 0.8)
    chop = sample_fbm(chop_octaves, fx, fy)
    return swell_a * 0.20 + swell_b * 0.16 + chop * 0.62


def make_water():
    albedo, height, rough = new_state()
    rng = random.Random("water")
    warp_octaves = build_fbm_octaves(random.Random("water_warp"), WATER_WARP_SPEC)
    chop_octaves = build_fbm_octaves(random.Random("water_chop"), WATER_CHOP_SPEC)
    H, S, V = 205, 0.55, 0.55  # unchanged HSV target -- see PALETTE.md
    BR = 0.09
    for y in range(SIZE):
        fy = y / SIZE
        for x in range(SIZE):
            fx = x / SIZE
            ripple = water_ripple(warp_octaves, chop_octaves, fx, fy)
            v_ref = V
            v = v_ref + ripple * 0.15 + jitter(rng, 0, 0.012)
            s = S - abs(ripple) * 0.07 + jitter(rng, 0, 0.02)
            h = H + jitter(rng, 0, 3)
            rr = BR
            # height always follows the smooth ripple field, glint or not --
            # an earlier version punched an extra one-off height spike into
            # glint pixels, which a normal map turns into a sharp colored
            # +-shaped artifact (steep opposing gradients either side of one
            # pixel); at 2x2 tile zoom that read as its own little scattered
            # dot lattice on top of the wave bands. Glints stay visible via
            # albedo brightness + near-mirror roughness only.
            h_val = clamp01(0.5 + ripple * 0.34)
            if rng.random() < 0.016:
                v += 0.24  # sun-glint highlight, near-mirror
                s -= 0.18
                emit(albedo, height, rough, x, y, h, s, v, v_ref, rr, rough_override=0.03)
            else:
                emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
            height[y][x] = h_val
    return albedo, height, rough


def build_water_before_after(old_albedo_path):
    cell = 256
    pad, label_h = 10, 22
    cols = [
        ("water.png BEFORE", Image.open(old_albedo_path).convert("RGB")),
        ("water.png AFTER", Image.open(BLOCKS_DIR / "water.png").convert("RGB")),
        ("water_n.png AFTER", Image.open(BLOCKS_DIR / "water_n.png").convert("RGB")),
        ("water_r.png AFTER", Image.open(BLOCKS_DIR / "water_r.png").convert("RGB")),
    ]
    W = len(cols) * (cell + pad) + pad
    Hh = cell + label_h + 2 * pad
    sheet = Image.new("RGB", (W, Hh), (18, 19, 24))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 14)
    except Exception:
        font = ImageFont.load_default()
    for i, (label, im) in enumerate(cols):
        x0 = pad + i * (cell + pad)
        big = im.resize((cell, cell), Image.NEAREST)
        sheet.paste(big, (x0, label_h + pad))
        draw.rectangle([x0, label_h + pad, x0 + cell - 1, label_h + pad + cell - 1], outline=(70, 70, 78), width=1)
        draw.text((x0, 2), label, fill=(230, 230, 230), font=font)
    out = BLOCKS_DIR / "water_before_after.png"
    sheet.save(out)
    print(f"water before/after -> {out}")
    return out


# =====================================================================
# 3) VEGETATION (assets/textures/blocks/vegetation/)
# =====================================================================

def blank_paint_state():
    return {
        "A": [[0.0] * SIZE for _ in range(SIZE)],
        "CR": [[0.0] * SIZE for _ in range(SIZE)],
        "CG": [[0.0] * SIZE for _ in range(SIZE)],
        "CB": [[0.0] * SIZE for _ in range(SIZE)],
        "H": [[0.5] * SIZE for _ in range(SIZE)],
        "R": [[0.5] * SIZE for _ in range(SIZE)],
    }


def paint(state, x, y, inside, rgb01, height_val=None, rough_val=None):
    if inside <= 0.0008:
        return
    a0 = state["A"][y][x]
    a1 = inside + a0 * (1 - inside)
    if a1 <= 0.0008:
        return
    r, g, b = rgb01
    cr0, cg0, cb0 = state["CR"][y][x], state["CG"][y][x], state["CB"][y][x]
    cr1 = (r * inside + cr0 * a0 * (1 - inside)) / a1
    cg1 = (g * inside + cg0 * a0 * (1 - inside)) / a1
    cb1 = (b * inside + cb0 * a0 * (1 - inside)) / a1
    state["CR"][y][x], state["CG"][y][x], state["CB"][y][x] = cr1, cg1, cb1
    state["A"][y][x] = a1
    if height_val is not None:
        state["H"][y][x] = state["H"][y][x] * (1 - inside) + height_val * inside
    if rough_val is not None:
        state["R"][y][x] = state["R"][y][x] * (1 - inside) + rough_val * inside


def state_to_images(state):
    albedo = Image.new("RGBA", (SIZE, SIZE))
    for y in range(SIZE):
        for x in range(SIZE):
            a = clamp01(state["A"][y][x])
            r = clamp01(state["CR"][y][x])
            g = clamp01(state["CG"][y][x])
            b = clamp01(state["CB"][y][x])
            albedo.putpixel((x, y), (round(r * 255), round(g * 255), round(b * 255), round(a * 255)))
    n_img = normal_from_height(state["H"], wrap=False)
    r_img = roughness_img(state["R"])
    return albedo, n_img, r_img


def capsule_mask(x, y, ax, ay, bx, by, r0, r1, edge=1.0):
    abx, aby = bx - ax, by - ay
    apx, apy = x - ax, y - ay
    ab2 = abx * abx + aby * aby
    t = 0.0 if ab2 <= 1e-9 else clamp01((apx * abx + apy * aby) / ab2)
    cx, cy = ax + t * abx, ay + t * aby
    d = math.hypot(x - cx, y - cy)
    r = r0 + (r1 - r0) * t
    return smoothstep(r + edge, r - edge, d), t


def circle_mask(x, y, cx, cy, r, edge=1.0):
    d = math.hypot(x - cx, y - cy)
    return smoothstep(r + edge, r - edge, d)


def petal_mask(x, y, cx, cy, k, base, amp, exp_, phase, edge=1.2):
    dx, dy = x - cx, y - cy
    r = math.hypot(dx, dy)
    theta = math.atan2(dy, dx)
    lobe = max(0.0, math.cos(k * theta + phase))
    pr = base + amp * (lobe ** exp_)
    inside = smoothstep(pr + edge, pr - edge, r)
    frac = 0.0 if pr <= 0 else clamp01(r / pr)
    return inside, frac


# ---- shared stem/leaf painter for the three flowers -----------------------

def paint_stem_and_leaves(state, rng, stem_a, stem_b, leaf_specs, stem_hsv, leaf_hsv):
    for y in range(SIZE):
        for x in range(SIZE):
            inside, t = capsule_mask(x, y, *stem_a, *stem_b, 1.6, 1.0)
            if inside > 0:
                h, s, v = stem_hsv
                v2 = v + jitter(rng, 0, 0.03)
                paint(state, x, y, inside, hsv01(h, s, v2), height_val=0.56, rough_val=0.72)
            for (la, lb, r0, r1) in leaf_specs:
                li, lt = capsule_mask(x, y, *la, *lb, r0, r1, edge=1.3)
                if li > 0:
                    h, s, v = leaf_hsv
                    v2 = v + (0.5 - lt) * 0.06 + jitter(rng, 0, 0.03)
                    paint(state, x, y, li, hsv01(h, s, v2), height_val=0.58, rough_val=0.70)


def make_flower_red():
    state = blank_paint_state()
    rng = random.Random("flower_red")
    paint_stem_and_leaves(
        state, rng, (32, 61), (33, 26),
        [((33, 50), (24, 44), 2.6, 0.6), ((33, 42), (41, 37), 2.2, 0.5)],
        (108, 0.55, 0.28), (105, 0.50, 0.33),
    )
    cx, cy = 33, 20
    for y in range(SIZE):
        for x in range(SIZE):
            inside, frac = petal_mask(x, y, cx, cy, k=6, base=7.0, amp=6.5, exp_=0.55, phase=0.3, edge=1.2)
            if inside > 0:
                v = 0.58 + frac * 0.18 + jitter(rng, 0, 0.03)
                s = 0.80 - frac * 0.06
                paint(state, x, y, inside, hsv01(6 + jitter(rng, 0, 3), s, v), height_val=0.62 + frac * 0.10, rough_val=0.58)
            c_in = circle_mask(x, y, cx, cy, 2.6, edge=0.8)
            if c_in > 0:
                paint(state, x, y, c_in, hsv01(15, 0.45, 0.14 + jitter(rng, 0, 0.02)), height_val=0.40, rough_val=0.75)
            fleck = circle_mask(x, y, cx, cy, 3.6, edge=0.6) - circle_mask(x, y, cx, cy, 2.6, edge=0.6)
            if fleck > 0 and rng.random() < 0.5:
                paint(state, x, y, max(0.0, fleck) * 0.6, hsv01(10, 0.3, 0.05), height_val=0.40, rough_val=0.8)
    return state_to_images(state)


def make_flower_pink():
    state = blank_paint_state()
    rng = random.Random("flower_pink")
    paint_stem_and_leaves(
        state, rng, (32, 61), (32, 27),
        [((32, 49), (23, 45), 2.4, 0.6), ((32, 40), (40, 36), 2.0, 0.5)],
        (108, 0.52, 0.30), (104, 0.48, 0.35),
    )
    cx, cy = 32, 21
    for y in range(SIZE):
        for x in range(SIZE):
            inside, frac = petal_mask(x, y, cx, cy, k=5, base=7.5, amp=5.5, exp_=0.35, phase=0.5, edge=1.3)
            if inside > 0:
                v = 0.82 + frac * 0.10 + jitter(rng, 0, 0.02)
                s = 0.46 - frac * 0.10
                paint(state, x, y, inside, hsv01(332 + jitter(rng, 0, 4), s, v), height_val=0.60 + frac * 0.08, rough_val=0.52)
            if circle_mask(x, y, cx, cy, 1.6, edge=0.6) > 0:
                paint(state, x, y, circle_mask(x, y, cx, cy, 1.6, edge=0.6), hsv01(60, 0.15, 0.30), height_val=0.42, rough_val=0.7)
    for i in range(5):
        ang = i * (2 * math.pi / 5) + 0.3
        sx, sy = cx + math.cos(ang) * 2.2, cy + math.sin(ang) * 2.2
        for y in range(max(0, round(sy) - 2), min(SIZE, round(sy) + 3)):
            for x in range(max(0, round(sx) - 2), min(SIZE, round(sx) + 3)):
                m = circle_mask(x, y, sx, sy, 0.9, edge=0.5)
                if m > 0:
                    paint(state, x, y, m, hsv01(50, 0.70, 0.85), height_val=0.65, rough_val=0.5)
    return state_to_images(state)


def make_flower_white():
    state = blank_paint_state()
    rng = random.Random("flower_white")
    paint_stem_and_leaves(
        state, rng, (32, 61), (33, 28),
        [((33, 51), (25, 46), 2.4, 0.5)],
        (108, 0.50, 0.27), (104, 0.46, 0.32),
    )
    cx, cy = 33, 22
    for y in range(SIZE):
        for x in range(SIZE):
            inside, frac = petal_mask(x, y, cx, cy, k=13, base=5.0, amp=9.0, exp_=4.0, phase=0.0, edge=1.0)
            if inside > 0:
                v = 0.92 + frac * 0.05 + jitter(rng, 0, 0.02)
                paint(state, x, y, inside, hsv01(55, 0.06, v), height_val=0.58 + frac * 0.06, rough_val=0.55)
            d_in = circle_mask(x, y, cx, cy, 4.5, edge=1.0)
            if d_in > 0:
                speck = 0.06 if ((x * 7 + y * 5) % 5 == 0) else 0.0
                v = 0.82 - speck + jitter(rng, 0, 0.02)
                paint(state, x, y, d_in, hsv01(48, 0.78, v), height_val=0.66, rough_val=0.60)
    return state_to_images(state)


def make_foliage_bush():
    state = blank_paint_state()
    rng = random.Random("foliage_bush")
    lobes = []
    for _ in range(7):
        lobes.append((rng.uniform(14, 50), rng.uniform(26, 60), rng.uniform(9, 14)))
    # small soft leaf-gap stipple, NOT a few big dark craters (those read as
    # eyes/a face at a glance -- keep them small, numerous and shallow)
    gaps = []
    for _ in range(16):
        gaps.append((rng.uniform(15, 49), rng.uniform(26, 59), rng.uniform(1.0, 1.9)))
    # tiny accent berries kept off-centre and away from any near-mirror pair
    # (a symmetric pair of dots in the upper-middle reads as eyes -- scatter
    # them low/along the rim instead)
    berries = []
    for _ in range(3):
        berries.append((rng.uniform(16, 48), rng.uniform(48, 58)))

    H, S, V = 116, 0.55, 0.40
    for y in range(SIZE):
        for x in range(SIZE):
            best = 0.0
            for (lcx, lcy, lr) in lobes:
                m = circle_mask(x, y, lcx, lcy, lr, edge=1.5)
                if m > best:
                    best = m
            if best <= 0:
                continue
            cluster = math.sin(x * 0.35 + 1.7) * math.cos(y * 0.32 + 0.4)
            v_ref = V + cluster * 0.10
            v = v_ref + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.06)
            gap_reduce = 0.0
            for (gcx, gcy, gr) in gaps:
                gm = circle_mask(x, y, gcx, gcy, gr, edge=1.2)
                if gm > gap_reduce:
                    gap_reduce = gm
            inside = best * (1 - 0.85 * gap_reduce)
            if gap_reduce > 0.3:
                v -= 0.16
            rough_v = 0.82 - gap_reduce * 0.10
            if inside > 0:
                paint(state, x, y, inside, hsv01(H + jitter(rng, 0, 5), s, v), height_val=0.5 + (v - v_ref) * 1.8, rough_val=rough_v)
    for (bx, by) in berries:
        for y in range(max(0, round(by) - 2), min(SIZE, round(by) + 3)):
            for x in range(max(0, round(bx) - 2), min(SIZE, round(bx) + 3)):
                m = circle_mask(x, y, bx, by, 1.2, edge=0.5)
                if m > 0 and state["A"][y][x] > 0.4:
                    paint(state, x, y, m, hsv01(350, 0.60, 0.55), height_val=0.65, rough_val=0.45)
    return state_to_images(state)


def make_grass_tall():
    state = blank_paint_state()
    rng = random.Random("grass_tall")
    blade_xs = [16, 22, 28, 34, 40, 46, 51]
    for bi, bx0 in enumerate(blade_xs):
        base = (bx0 + jitter(rng, 0, 1.5), 62 + jitter(rng, 0, 1.0))
        sway = jitter(rng, 0, 7.0)
        tip_y = rng.uniform(9, 23)
        mid = (base[0] + sway * 0.55, (base[1] + tip_y) / 2 + jitter(rng, 0, 2))
        tip = (base[0] + sway, tip_y)
        for y in range(SIZE):
            for x in range(SIZE):
                i1, t1 = capsule_mask(x, y, base[0], base[1], mid[0], mid[1], 2.0, 1.1, edge=0.9)
                i2, t2 = capsule_mask(x, y, mid[0], mid[1], tip[0], tip[1], 1.1, 0.4, edge=0.9)
                inside, t = (i1, t1 * 0.5) if i1 >= i2 else (i2, 0.5 + t2 * 0.5)
                if inside <= 0:
                    continue
                h = lerp(100, 82, t)
                s = 0.52
                v = lerp(0.27, 0.58, t) + jitter(rng, 0, 0.03)
                paint(state, x, y, inside, hsv01(h, s, v), height_val=0.5 + (0.5 - t) * 0.12, rough_val=0.84)
    return state_to_images(state)


def make_leaf_birch():
    state = blank_paint_state()
    rng = random.Random("leaf_birch")
    twig_a, twig_b = (20, 58), (46, 22)
    leaf_positions = [0.12, 0.26, 0.38, 0.50, 0.60, 0.70, 0.80, 0.90]
    for y in range(SIZE):
        for x in range(SIZE):
            inside, t = capsule_mask(x, y, *twig_a, *twig_b, 1.3, 0.7, edge=0.8)
            if inside > 0:
                paint(state, x, y, inside, hsv01(30, 0.25, 0.30 + jitter(rng, 0, 0.02)), height_val=0.55, rough_val=0.70)
    dx, dy = twig_b[0] - twig_a[0], twig_b[1] - twig_a[1]
    tlen = math.hypot(dx, dy)
    ux, uy = dx / tlen, dy / tlen
    px, py = -uy, ux
    for i, t in enumerate(leaf_positions):
        cxp = twig_a[0] + dx * t
        cyp = twig_a[1] + dy * t
        side = 1 if i % 2 == 0 else -1
        lcx = cxp + px * side * 5.0
        lcy = cyp + py * side * 5.0
        ang = math.atan2(py * side, px * side)
        ca, sa = math.cos(ang), math.sin(ang)
        rx, ry = 5.2, 3.4
        h_leaf = 86 + jitter(rng, 0, 4)
        for yy in range(max(0, round(lcy) - 8), min(SIZE, round(lcy) + 9)):
            for xx in range(max(0, round(lcx) - 8), min(SIZE, round(lcx) + 9)):
                lx = (xx - lcx) * ca + (yy - lcy) * sa
                ly = -(xx - lcx) * sa + (yy - lcy) * ca
                ed = math.hypot(lx / rx, ly / ry)
                inside = smoothstep(1.14, 0.86, ed)
                if inside <= 0:
                    continue
                v = 0.62 + jitter(rng, 0, 0.03)
                paint(state, xx, yy, inside, hsv01(h_leaf, 0.50, v), height_val=0.58, rough_val=0.66)
                vein = smoothstep(1.6, 0.0, abs(ly) / max(0.4, ry) * 4.0) if abs(lx) < rx * 0.9 else 0.0
                if vein > 0:
                    paint(state, xx, yy, inside * vein * 0.5, hsv01(h_leaf, 0.50, v - 0.12), height_val=0.46, rough_val=0.66)
    return state_to_images(state)


def make_leaf_pine():
    state = blank_paint_state()
    rng = random.Random("leaf_pine")
    twig_a, twig_b = (18, 60), (44, 20)
    for y in range(SIZE):
        for x in range(SIZE):
            inside, t = capsule_mask(x, y, *twig_a, *twig_b, 1.2, 0.6, edge=0.8)
            if inside > 0:
                paint(state, x, y, inside, hsv01(28, 0.30, 0.28 + jitter(rng, 0, 0.02)), height_val=0.55, rough_val=0.70)
    dx, dy = twig_b[0] - twig_a[0], twig_b[1] - twig_a[1]
    tlen = math.hypot(dx, dy)
    base_ang = math.atan2(dy, dx)
    burst_ts = [0.10, 0.24, 0.38, 0.50, 0.62, 0.74, 0.86, 0.96]
    for bi, t in enumerate(burst_ts):
        bx = twig_a[0] + dx * t
        by = twig_a[1] + dy * t
        n_needles = 11
        for ni in range(n_needles):
            spread = -1.15 + (2.30 * ni / (n_needles - 1))
            ang = base_ang + math.pi / 2 + spread
            length = rng.uniform(8, 15)
            ex = bx + math.cos(ang) * length
            ey = by + math.sin(ang) * length
            lit = rng.random() < 0.35
            v = (0.42 if lit else 0.30) + jitter(rng, 0, 0.03)
            h = 150 + jitter(rng, 0, 4)
            for yy in range(max(0, round(min(by, ey)) - 1), min(SIZE, round(max(by, ey)) + 2)):
                for xx in range(max(0, round(min(bx, ex)) - 1), min(SIZE, round(max(bx, ex)) + 2)):
                    inside, tt = capsule_mask(xx, yy, bx, by, ex, ey, 0.9, 0.15, edge=0.6)
                    if inside > 0:
                        paint(state, xx, yy, inside, hsv01(h, 0.55, v), height_val=0.5 + (1 - tt) * 0.08, rough_val=0.68)
    return state_to_images(state)


VEGETATION = [
    ("flower_red", make_flower_red),
    ("flower_pink", make_flower_pink),
    ("flower_white", make_flower_white),
    ("foliage_bush", make_foliage_bush),
    ("grass_tall", make_grass_tall),
    ("leaf_birch", make_leaf_birch),
    ("leaf_pine", make_leaf_pine),
]


def build_vegetation_contact_sheet():
    cols = 3
    rows = len(VEGETATION)
    cell = 176
    label_h = 20
    pad = 8
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

    headers = ["albedo (on checker)", "normal", "roughness"]
    for c, hname in enumerate(headers):
        draw.text((pad + c * (cell + pad), 2), hname, fill=(210, 210, 220), font=hdr_font)

    checker = Image.new("RGB", (cell, cell))
    cd = ImageDraw.Draw(checker)
    step = cell // 16
    for j in range(16):
        for i in range(16):
            col = (60, 60, 66) if (i + j) % 2 == 0 else (40, 40, 45)
            cd.rectangle([i * step, j * step, i * step + step, j * step + step], fill=col)

    for i, (name, _) in enumerate(VEGETATION):
        y0 = label_h + pad + i * (cell + label_h + pad)
        albedo = Image.open(VEG_DIR / f"{name}.png").convert("RGBA").resize((cell, cell), Image.NEAREST)
        tile0 = checker.copy()
        tile0.paste(albedo, (0, 0), albedo)
        normal = Image.open(VEG_DIR / f"{name}_n.png").convert("RGB").resize((cell, cell), Image.NEAREST)
        rough = Image.open(VEG_DIR / f"{name}_r.png").convert("RGB").resize((cell, cell), Image.NEAREST)
        for c, tile in enumerate([tile0, normal, rough]):
            x0 = pad + c * (cell + pad)
            sheet.paste(tile, (x0, y0))
            draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        draw.text((pad, y0 + cell + 2), name, fill=(230, 230, 230), font=font)

    out = VEG_DIR / "vegetation_contact_sheet.png"
    sheet.save(out)
    print(f"vegetation contact sheet -> {out}")
    return out


# =====================================================================
# combined overview
# =====================================================================

def build_master_sheet(sky_sheet_path, water_sheet_path, veg_sheet_path):
    def load_w(path, w):
        im = Image.open(path).convert("RGB")
        ratio = w / im.width
        return im.resize((w, round(im.height * ratio)), Image.NEAREST)

    W = 1100
    pad = 14
    imgs = [load_w(p, W) for p in (sky_sheet_path, water_sheet_path, veg_sheet_path)]
    Hh = sum(im.height for im in imgs) + pad * (len(imgs) + 1) + 3 * 26
    sheet = Image.new("RGB", (W + pad * 2, Hh), (12, 13, 16))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arialbd.ttf", 18)
    except Exception:
        font = ImageFont.load_default()
    labels = ["SKY", "WATER (before -> after)", "VEGETATION (cross-sprites)"]
    y = pad
    for label, im in zip(labels, imgs):
        draw.text((pad, y), label, fill=(240, 220, 190), font=font)
        y += 26
        sheet.paste(im, (pad, y))
        y += im.height + pad
    out = ROOT / "assets" / "textures" / "env_pack_contact_sheet.png"
    sheet.save(out)
    print(f"MASTER contact sheet -> {out}")
    return out


def main():
    # ---- sky ----
    save(make_sky_gradient(), SKY_DIR, "sky_gradient_sunset")
    save(make_sky_clouds(), SKY_DIR, "sky_clouds")
    save(make_sun_disk(), SKY_DIR, "sun_disk")
    print("sky: sky_gradient_sunset.png, sky_clouds.png, sun_disk.png")
    sky_sheet = build_sky_contact_sheet()

    # ---- water re-pass ----
    old_water_backup = BLOCKS_DIR / "_water_before_repass.png"
    if not old_water_backup.exists():
        Image.open(BLOCKS_DIR / "water.png").save(old_water_backup)
    albedo, height, rough = make_water()
    save(albedo, BLOCKS_DIR, "water")
    save(normal_from_height(height, wrap=True), BLOCKS_DIR, "water", "_n")
    save(roughness_img(rough), BLOCKS_DIR, "water", "_r")
    print(f"water: re-authored, avg {avg_hex(albedo)}")
    water_sheet = build_water_before_after(old_water_backup)

    # ---- vegetation ----
    for name, fn in VEGETATION:
        albedo, n_img, r_img = fn()
        save(albedo, VEG_DIR, name)
        save(n_img, VEG_DIR, name, "_n")
        save(r_img, VEG_DIR, name, "_r")
        print(f"vegetation: {name:14s} avg {avg_hex(albedo)}")
    veg_sheet = build_vegetation_contact_sheet()

    build_master_sheet(sky_sheet, water_sheet, veg_sheet)


if __name__ == "__main__":
    main()
