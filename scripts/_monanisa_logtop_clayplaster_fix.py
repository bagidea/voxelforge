"""
Monanisa / art lane -- 2026-08-21: two live artifacts the 2026-08-20 material-
lock pass never touched (its TARGETS list was wood/brick/stone/dirt/sand only).

Found by shooting a real in-game frame on beach_dusk.json with the material
system on (target-poppy/release/voxelforge.exe, BLOCK_PBR authored confirmed
in the log) while verifying Director's "textures still not realistic" note:

1. oak_log_top.png -- a perfect 14-spoke pinwheel/starburst radiates from the
   tile centre to the bark rim. Root cause: _pixel_blocks_gen64.py::
   make_log_top()'s "crack" term is `abs(sin(angle * 7)) > 0.985`, a clean
   n=7 (doubled by abs() to 14) periodic angular harmonic -- structurally the
   same class of bug as the paint_blobs() polar-rose "star-splat" fixed in
   be365e9, just in a different function nobody re-audited afterward. Real
   log radial checks (drying cracks) are few, irregular in count/width/
   length/spacing -- not evenly-spaced spokes reaching a uniform radius.
   Fix: replace the periodic term with 5-7 explicit checks, each an
   independent (angle, angular half-width, start radius, end radius) drawn
   from its own RNG stream -- irregular by construction, no harmonic to snap
   to a count.

2. clay_plaster.png -- red + green blob disks read as a floral print at
   in-game distance. Root cause: _pixel_blocks_gen64_v4_kevin.py::
   kevin_clay_plaster() calls `red_clay(r_range=(9,15), exclude=y<0.45*SIZE)`
   and a damp/mold `paint_blobs(h=100, r_range=(9,15), exclude=y<0.50*SIZE)`
   -- the exact bug documented in 54d4d81 (scatter_blobs' exclude only gates
   the blob CENTER; paint_blobs then paints the full radius regardless), just
   never applied to this tile because clay_plaster wasn't in that pass's
   TARGETS list. At r=15 on a 64px tile a "confined to the lower wall" blob
   covers up to 47% of the tile height regardless of the exclude line.
   Fix: same technique as 54d4d81 -- masked_speckle() (reused verbatim from
   _monanisa_material_lock_pass.py, untouched) gates every texel individually
   against the lower-wall mask, so the red-ochre / damp-mold accents can
   never rise above the line no matter how large density gets. Hue family
   (h=352 red-ochre "hue bin opener", h=100 green damp) is kept exactly --
   this is a containment fix, not a recolour; only the shape + total coverage
   moved (coverage tuned back to roughly what the blob version painted, so
   any palette-breadth grading that depends on this hue bin being populated
   still sees it).

Per tile-zoom-check-procedural: verified 2x2-tiled zoom on both, no new
periodic/seam artifact. Per guard-generator-output-procedural-asset: writes
to SCRATCH only; promote() is a separate explicit step run after visual
review. Never touches _pixel_blocks_gen64*.py or any live asset directly.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402
import _pixel_blocks_gen64_v3_palette as v3  # noqa: E402
import _monanisa_material_lock_pass as lock  # noqa: E402

SIZE = base.SIZE
jitter = base.jitter
emit = base.emit
new_state = base.new_state
masked_speckle = lock.masked_speckle
paint_accent = v3.paint_accent

SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_logtop_clayplaster_fix"
SCRATCH.mkdir(parents=True, exist_ok=True)
_LIVE_DIR = (base.ROOT / "assets" / "textures" / "blocks").resolve()
if SCRATCH.resolve() == _LIVE_DIR or _LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")

TILES = ("oak_log_top", "clay_plaster")


def save_scratch(img, name, suffix=""):
    p = SCRATCH / f"{name}{suffix}.png"
    img.save(p)
    return p


# ------------------------------------------------------------- oak_log_top
def make_log_top_fixed():
    albedo, height, rough = new_state()
    rng = random.Random("oak_log_top")
    cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
    H = 33
    BR = 0.60

    crng = random.Random("oak_log_top-checks-irregular")
    checks = []
    for _ in range(crng.randint(5, 7)):
        ang0 = crng.uniform(0, math.tau)
        half_w = crng.uniform(0.035, 0.075)
        r0 = crng.uniform(4, 9)
        r1 = crng.uniform(16, 28)
        checks.append((ang0, half_w, r0, r1))

    for y in range(SIZE):
        for x in range(SIZE):
            d = math.hypot(x - cx, y - cy)
            ring = math.sin(d * 0.42) * 0.5 + 0.5
            angle = math.atan2(y - cy, x - cx)
            crack = 0.0
            for ang0, half_w, r0, r1 in checks:
                da = abs((angle - ang0 + math.pi) % math.tau - math.pi)
                if da < half_w and r0 <= d <= r1:
                    crack = 1.0
                    break
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


def make_log_top_v3_fixed():
    """Same wrapper _monanisa_material_lock_pass.make_log_top_v3() applies,
    just fed the irregular-crack base instead of base.make_log_top()."""
    albedo, height, rough = make_log_top_fixed()
    r = random.Random("oak_log_top-lock")

    def ring_band(x, y):
        cx, cy = (SIZE - 1) / 2, (SIZE - 1) / 2
        d = math.hypot(x - cx, y - cy)
        return 10 < d < 24

    blobs = v3.scatter_blobs(r, 2, (2.0, 3.2), exclude=lambda x, y: not ring_band(x, y))
    v3.paint_blobs(r, albedo, height, rough, blobs, h=30, s_range=(0.10, 0.16), v=0.32,
                    target_height=0.34, target_rough=0.70, max_blend=0.75, h_jitter=4)
    return albedo, height, rough


# ------------------------------------------------------------- clay_plaster
def masked_speckle_fade(rng, albedo, height, rough, fade_fn, h, s_range, v,
                         target_height, target_rough, peak_density,
                         blend_range, h_jitter=6):
    """Like masked_speckle(), but density ramps continuously via
    fade_fn(x, y) -> 0..1 instead of a hard mask boundary -- a stain that
    THINS OUT toward the floor line, not a band with a razor edge."""
    for y in range(SIZE):
        for x in range(SIZE):
            w = fade_fn(x, y)
            if w <= 0.0:
                continue
            if rng.random() >= peak_density * w:
                continue
            hh = h + jitter(rng, 0, h_jitter)
            ss = s_range[0] + rng.random() * (s_range[1] - s_range[0])
            blend = (blend_range[0] + rng.random() * (blend_range[1] - blend_range[0])) * (0.5 + 0.5 * w)
            paint_accent(albedo, height, rough, x, y, hh, ss, v, target_height, target_rough, blend)


def make_clay_plaster_fixed():
    albedo, height, rough = base.make_clay_plaster()
    v3.polish_clay_plaster(albedo, height, rough)

    # Rising-damp rust stain: thin colonnades climbing from the floor line,
    # not a flat-topped band -- density and reach both vary per column so
    # the top edge is ragged (real capillary staining), and it fades to
    # nothing well before mid-wall instead of stopping on a hard line.
    col_rng = random.Random("clay_plaster-rust-columns")
    floor_y = SIZE * 0.78
    reach = [col_rng.uniform(0.10, 0.30) * SIZE for _ in range(SIZE)]

    def rust_fade(x, y):
        top = floor_y - reach[x]
        if y < top:
            return 0.0
        span = floor_y - top
        return min(1.0, (y - top) / max(span, 1.0))

    rust_rng = random.Random("clay_plaster-rust-masked")
    masked_speckle_fade(rust_rng, albedo, height, rough, rust_fade,
                         h=352, s_range=(0.48, 0.60), v=0.56,
                         target_height=0.46, target_rough=0.64,
                         peak_density=0.20, blend_range=(0.30, 0.55), h_jitter=6)

    # A couple of small damp/mould patches tucked in the bottom corners only,
    # not a second wide band layered on top of the rust.
    def corner_fade(x, y):
        if y < SIZE * 0.72:
            return 0.0
        near_left = max(0.0, 1.0 - x / (SIZE * 0.22))
        near_right = max(0.0, 1.0 - (SIZE - 1 - x) / (SIZE * 0.22))
        return max(near_left, near_right)

    damp_rng = random.Random("clay_plaster-damp-masked")
    masked_speckle_fade(damp_rng, albedo, height, rough, corner_fade,
                         h=100, s_range=(0.16, 0.24), v=0.58,
                         target_height=0.44, target_rough=0.74,
                         peak_density=0.14, blend_range=(0.30, 0.50), h_jitter=6)

    return albedo, height, rough


MAKE = {
    "oak_log_top": make_log_top_v3_fixed,
    "clay_plaster": make_clay_plaster_fixed,
}


def main():
    for name in TILES:
        albedo, height, rough = MAKE[name]()
        assert albedo.size == (SIZE, SIZE)
        save_scratch(albedo, name)
        save_scratch(base.normal_from_height(height), name, "_n")
        save_scratch(base.roughness_img(rough), name, "_r")
        print(f"wrote {name} (+ _n/_r) to {SCRATCH}")


def promote():
    """Explicit promotion step -- copies scratch <name>.png/_n/_r over the
    live assets/textures/blocks/ triplet, one tile at a time. Run only after
    visual review. Usage: --promote oak_log_top | clay_plaster | all"""
    import shutil
    which = sys.argv[2] if len(sys.argv) > 2 else "all"
    names = TILES if which == "all" else (which,)
    for name in names:
        for suffix in ("", "_n", "_r"):
            src = SCRATCH / f"{name}{suffix}.png"
            dst = _LIVE_DIR / f"{name}{suffix}.png"
            shutil.copyfile(src, dst)
            print(f"promoted {src} -> {dst}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--promote":
        promote()
    else:
        main()
