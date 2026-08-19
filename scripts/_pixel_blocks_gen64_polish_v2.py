"""
Monanisa / art lane — polish pass v2 on 4 flagged 64x64 blocks.

Director's review of the 64px PBR set flagged 4 textures as not reading right
up close:
  - leaves        : the x/y-separable sin(x)*cos(y) cluster field reads as a
                     "woven fabric" plaid, not overlapping foliage clumps.
  - oak_log_top    : the crack pattern (14 evenly-spaced radial spokes from
                     abs(sin(angle*7))>0.985) reads as a "bicycle wheel", and
                     the rings are perfectly concentric/perfectly periodic.
  - snow, red_sand : too flat at 64px — no visible grain/ripple up close.

This is a STAGING pass, not a promotion: outputs go to
assets/textures/blocks/_polish_v2/ (base + _n + _r per block, same 64x64,
identical filenames to the live set) so the live textures Poppy is shooting
A/B against are never touched. Director promotes by copying these over.

Reuses new_state/emit/normal_from_height/roughness_img/save/avg_hex from the
64px generator so the height->(normal,roughness) derivation stays identical
to the rest of the set; only the 4 albedo pattern functions are rewritten.
"""

import math
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _pixel_blocks_gen64 import (  # noqa: E402
    SIZE,
    avg_hex,
    clamp01,
    emit,
    jitter,
    new_state,
    normal_from_height,
    roughness_img,
)

ROOT = Path(__file__).resolve().parent.parent
LIVE_DIR = ROOT / "assets" / "textures" / "blocks"
OUT_DIR = LIVE_DIR / "_polish_v2"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def save(img, name, suffix=""):
    path = OUT_DIR / f"{name}{suffix}.png"
    img.save(path)
    return path


# ------------------------------------------------------------------- leaves
def make_leaves():
    """Overlapping foliage clumps (Worley-style blob field) with irregular,
    randomly-sized light gaps — replaces the sin(x)*cos(y) plaid grid, which
    is x/y-separable and therefore reads as a woven weft/warp pattern.

    v2b (first revision): the first pass used 15 blobs sized 7-15px on a
    64px tile — at that scale only ~3-4 blobs fit per tile, so the *same*
    peanut/figure-8 silhouette repeats identically whenever the block tiles
    across a canopy, reading as a stamped motif (confirmed by tiling the
    output 3x3 and eyeballing it — an unmistakable repeating blob shape).

    v2c (this revision): the v2b fix added a second *coarse* blob layer
    (8 blobs, r16-24) meant only as a subtle brightness undertone, but its
    low-frequency dips still dominated where the darkest "light gap" pixels
    landed — re-tiling v2b 3x3 still showed one big coherent dark void per
    tile, just as recognizable as the original. Dropped the coarse layer
    entirely: the whole read now comes from a single dense, small-radius
    Worley field (many more, smaller blobs than v2b's fine layer) so no
    single low-frequency shape survives to repeat, plus a low-amplitude
    continuous sine wobble (not blob-shaped, same technique as the snow/
    red_sand ripple fix) for a whisper of macro shading.

    v2c tone-fix (this revision, same field/radii/wobble as above): the first
    v2c cut read almost 3x darker at the median than live (p50 30 vs 80) with
    alpha=255 everywhere, so what looked like "light gaps" was actually near-
    black opaque texels — a burnt canopy, not dappled leaves. The light-gap
    penalty (v -= 0.20) stacked on top of an already-low base V=0.42 pushed
    the darkest clumps' cores under 0.15. Fix is a flat affine tone curve
    (lift blacks + pull contrast in around a mid-point) applied to the final
    v AND v_ref together *after* all the blob/gap/crown/wobble math — so the
    Worley field, blob radii/count, and wobble frequencies are untouched, and
    because the curve is affine, dv = v - v_ref (which drives the height/
    normal bump) just scales by the same contrast factor instead of changing
    shape. Constants tuned by brute-force sweeping lift/contrast/mid against
    the actual pixel luminance distribution until p5/p50/p95 landed inside
    the director's target band (p50 74-84, p5>=42, p95<=112)."""
    albedo, height, rough = new_state()
    rng = random.Random("leaves-v2c")
    H, S, V = 118, 0.62, 0.42
    BR = 0.80

    # affine tone curve: lift the black point + compress contrast around MID.
    # applied to v and v_ref alike (see docstring) so height/normal bump
    # amplitude scales down with contrast instead of being decoupled from it.
    TONE_LIFT, TONE_CONTRAST, TONE_MID = 0.18, 0.58, 0.33
    # the tone curve is affine, so it shrinks dv=(v-v_ref) - and therefore bump
    # depth (height/normal) - by the same TONE_CONTRAST factor as albedo
    # contrast. BUMP_COMP undoes that shrink on the height/normal path only:
    # v_ref (never read for albedo colour, only for dv) is stretched so
    # tone_map(v) - v_ref_bump ~= the pre-tone-curve dv again, while
    # tone_map(v) itself - the value that becomes the albedo pixel - is
    # untouched. The theoretical undo factor is 1/TONE_CONTRAST~=1.724, but
    # tone_map's own clamp01 caps how far the darkest gap pixels' tv can sit,
    # which caps the achievable dv for those pixels too; empirically 1.85 is
    # what it takes to land mean|xy| back in the director's 0.40-0.45 band
    # (1.724 measured 0.3913, just under the floor) while keeping z_mean<=0.88.
    BUMP_COMP = 1.85

    def tone_map(val):
        return clamp01((clamp01(val) - TONE_MID) * TONE_CONTRAST + TONE_MID + TONE_LIFT)

    def torus_dist(x, y, bx, by):
        dx = abs(x - bx)
        dx = min(dx, SIZE - dx)
        dy = abs(y - by)
        dy = min(dy, SIZE - dy)
        return math.hypot(dx, dy)

    fine_rng = random.Random("leaves-v2c-fine")
    fine_blobs = [
        (fine_rng.uniform(0, SIZE), fine_rng.uniform(0, SIZE), fine_rng.uniform(1.8, 3.6))
        for _ in range(150)
    ]

    def fine_field(x, y):
        total = 0.0
        for bx, by, br in fine_blobs:
            d = torus_dist(x, y, bx, by)
            f = 1.0 - d / br
            if f > 0:
                total += f ** 1.6
        return total

    field = [[fine_field(x, y) for x in range(SIZE)] for y in range(SIZE)]
    cmax = max(max(row) for row in field) or 1.0

    for y in range(SIZE):
        for x in range(SIZE):
            c = field[y][x] / cmax  # 0 = between clumps, high = clump core
            # frequencies are integer cycles over the 64px tile (2*pi*n/64) so
            # this wobble wraps seamlessly and can't itself create a tiling seam
            k = 2 * math.pi / SIZE
            wobble = math.sin(x * 4 * k + y * 2 * k) * 0.02 + math.sin(x * 2 * k - y * 5 * k + 0.8) * 0.015
            v_ref = V + (c - 0.4) * 0.22 + wobble
            v = v_ref + jitter(rng, 0, 0.04)
            s = S + jitter(rng, 0, 0.06)
            rr = BR
            if c < 0.12:
                v -= 0.20  # irregular light gap between clumps (blob-shaped, not on a grid)
                s -= 0.05
                rr += 0.05
            elif c > 0.75 and rng.random() < 0.35:
                v += 0.13  # sunlit clump crown
                rr -= 0.07
            tv = tone_map(v)
            tvr = tone_map(v_ref)
            tvr_bump = tv - (tv - tvr) * BUMP_COMP  # only feeds emit()'s dv, never the albedo pixel
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 5), s, tv, tvr_bump, rr)
    return albedo, height, rough


# --------------------------------------------------------------- oak_log_top
def make_log_top():
    """Concentric rings with per-ring offset centers (eccentric growth, like
    a real cross-section) and irregular spacing, plus only 1-2 radial
    checking cracks — replaces the abs(sin(angle*7)) crack test, which draws
    14 evenly-spaced spokes around the full circle (a bicycle wheel)."""
    albedo, height, rough = new_state()
    rng = random.Random("oak_log_top-v2")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
    H = 33
    BR = 0.60

    ring_rng = random.Random("oak_log_top-v2-rings")
    radii = []
    r = 2.0
    while r < 34:
        radii.append(r)
        r += ring_rng.uniform(2.2, 4.6)  # uneven ring-to-ring spacing
    ring_centers = [
        (cx + ring_rng.uniform(-2.5, 2.5), cy + ring_rng.uniform(-2.5, 2.5))
        for _ in radii
    ]

    n_cracks = ring_rng.choice([1, 2])
    crack_angles = [ring_rng.uniform(0, 2 * math.pi) for _ in range(n_cracks)]

    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy)
            angle = math.atan2(y - cy, x - cx)
            best = 1e9
            for (rcx, rcy), rad in zip(ring_centers, radii):
                dd = math.hypot(x - rcx, y - rcy)
                best = min(best, abs(dd - rad))
            ring = clamp01(1.0 - best / 1.6)
            crack = 0.0
            for ca in crack_angles:
                da = abs((angle - ca + math.pi) % (2 * math.pi) - math.pi)
                if da < 0.05 and d > 5:
                    crack = 1.0
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


# -------------------------------------------------------------------- snow
def make_snow():
    """Adds an angled wind-packed ripple (two frequencies) and a granular
    shadow-pocket category on top of the existing sparkle flecks — the old
    version had only a very low-amplitude dimple + rare sparkles, which reads
    flat at 64px."""
    albedo, height, rough = new_state()
    rng = random.Random("snow-v2")
    H, S, V = 45, 0.07, 0.94
    BR = 0.58
    ang = math.radians(22)
    ca, sa = math.cos(ang), math.sin(ang)
    for y in range(SIZE):
        for x in range(SIZE):
            rx = x * ca + y * sa
            ripple = math.sin(rx * 0.9) * 0.035 + math.sin(rx * 0.31 + 1.3) * 0.02
            dimple = math.sin(x * 0.20 + 0.4) * math.cos(y * 0.18 + 1.0) * 0.02
            v_ref = V + dimple + ripple
            v = v_ref + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.02)
            h = H
            rr = BR
            g = rng.random()
            if g < 0.10:
                v = min(1.0, v + 0.07)  # icy crystal facet catching light
                s = 0.03
                rr -= 0.20
            elif g < 0.17:
                v -= 0.06  # granular shadow pocket between crystals
                rr += 0.05
            elif g < 0.20:
                h = 205  # rare cool glint
                s = 0.05
            emit(albedo, height, rough, x, y, h, s, v, v_ref, rr)
    return albedo, height, rough


# --------------------------------------------------------------- red_sand
def make_red_sand():
    """Adds an angled dune-ripple (two frequencies) and a bleached-grain
    fleck category, and widens the per-pixel jitter — same "too flat"
    complaint as snow, same fix shape."""
    albedo, height, rough = new_state()
    rng = random.Random("red_sand-v2")
    H, S, V = 18, 0.55, 0.78
    BR = 0.92
    ang = math.radians(-15)
    ca, sa = math.cos(ang), math.sin(ang)
    for y in range(SIZE):
        for x in range(SIZE):
            rx = x * ca + y * sa
            dune = math.sin(rx * 0.55) * 0.045 + math.sin(rx * 0.19 + 0.7) * 0.02
            v_ref = V + dune
            v = v_ref + jitter(rng, 0, 0.05)
            s = S + jitter(rng, 0, 0.07)
            rr = BR
            g = rng.random()
            if g < 0.12:
                v -= 0.11  # grain shadow in ripple trough
                rr += 0.03
            elif g < 0.17:
                v += 0.09  # sunlit grain crest
                rr -= 0.06
            elif g < 0.20:
                s -= 0.15  # bleached mineral grain fleck
                v += 0.04
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 3), s, v, v_ref, rr)
    return albedo, height, rough


MATERIALS = [
    ("leaves", make_leaves),
    ("oak_log_top", make_log_top),
    ("snow", make_snow),
    ("red_sand", make_red_sand),
]


def build_before_after_sheet(avgs):
    cols = 2
    rows = len(MATERIALS)
    cell = 256
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

    draw.text((pad, 2), "BEFORE (live 64x64)", fill=(210, 190, 190), font=hdr_font)
    draw.text((pad + cell + pad, 2), "AFTER (_polish_v2)", fill=(190, 210, 190), font=hdr_font)

    for i, (name, _) in enumerate(MATERIALS):
        y0 = pad + label_h + i * (cell + label_h + pad)
        before = Image.open(LIVE_DIR / f"{name}.png").convert("RGB")
        before_big = before.resize((cell, cell), Image.NEAREST)
        after = Image.open(OUT_DIR / f"{name}.png").convert("RGB")
        after_big = after.resize((cell, cell), Image.NEAREST)
        x0 = pad
        sheet.paste(before_big, (x0, y0))
        draw.rectangle([x0, y0, x0 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        x1 = pad + cell + pad
        sheet.paste(after_big, (x1, y0))
        draw.rectangle([x1, y0, x1 + cell - 1, y0 + cell - 1], outline=(60, 62, 68), width=1)
        draw.text((x0, y0 - label_h + 2), f"{name}   after avg {avgs[name]}", fill=(230, 230, 230), font=font)

    out_path = OUT_DIR / "contact_sheet_v2.png"
    sheet.save(out_path)
    print(f"before/after v2 contact sheet -> {out_path}  ({W}x{Hh})")


def main():
    avgs = {}
    for name, fn in MATERIALS:
        albedo, height, rough = fn()
        assert albedo.size == (SIZE, SIZE)
        save(albedo, name)
        save(normal_from_height(height), name, "_n")
        save(roughness_img(rough), name, "_r")
        hexcode, _ = avg_hex(albedo)
        avgs[name] = hexcode
        print(f"{name:16s} v2 64x64 albedo {hexcode}  +_n +_r  -> {OUT_DIR}")

    build_before_after_sheet(avgs)


if __name__ == "__main__":
    main()
