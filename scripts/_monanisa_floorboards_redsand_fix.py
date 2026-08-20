"""
Monanisa / art lane -- 2026-08-21 (2): floorboards + red_sand still carry the
"floating blob disk" defect the material-lock pass claimed closed.

Review caught this in `docs/evidence/material-lock-2026-08-20/ingame-after-
fix2.png` itself -- the pathway tiles in that "after" screenshot are
floorboards/red_sand, and both still show the defect. Root cause confirmed by
opening the live PNGs directly: `_pixel_blocks_gen64_v4_kevin.py`'s
`kevin_floorboards()` / `kevin_red_sand()` layer LARGE (r 6-14px on a 64px
tile) `scatter_blobs()`+`paint_blobs()` accents on top of `_pixel_blocks_
gen64_v3_palette.py`'s already-correct small-grain accents (`polish_
floorboards`, `polish_red_sand`, r 0.6-2.8px) -- and neither tile was ever in
`_monanisa_material_lock_pass.py`'s TARGETS list (wood/brick/stone/dirt/sand
only), so this v4 layer reached the live asset unaudited. Exact same bug
class as 54d4d81/be365e9 and the 2026-08-21 oak_log_top/clay_plaster pass
(see `_monanisa_logtop_clayplaster_fix.py`): `scatter_blobs(exclude=...)`
only gates the blob CENTER, `paint_blobs()` then paints the full radius
regardless -- at r=8-14 on a 64px tile that's up to 44% of the tile per blob,
smooth-alpha-blended, so it reads as a floating colour disk instead of
material grain, whatever the exclude line intended to confine it to.

  floorboards: kevin's moss(h=98, r=8-13, exclude=near_seam) and
    red_clay(h=352, r=8-14, exclude=near_seam) -- green + red-ochre disks
    splashed across whole boards. v3's own polish_floorboards() algae layer
    (r=1.6-2.8) already opens the same h=98 bin at grain scale and is left
    untouched here.
    Fix: masked_speckle_fade() (reused from _monanisa_logtop_clayplaster_fix,
    untouched) gates every texel against a ragged upward creep from the
    REAL seam line (base.make_floorboards' `within >= board_h - 2`, i.e. the
    bottom 2px of each 16px board) -- moss and rust bleed can climb a few px
    into the board face, never past it, and the per-column reach varies so
    the top edge isn't a razor line. Same hue targets as kevin's version.

  red_sand: kevin's olivine/dark/pale blobs (h=110/15/25, r=4-11, NO exclude
    at all) are the exact same oversized-disk shape with no containment
    attempted. v3's polish_red_sand() already opens all three hue bins
    (mineral h=15, olivine h=110, shell h=25) at correct grain scale (r=0.6-
    1.3), the same technique proven on sand.png in
    _monanisa_sand_grain_restore.py. Fix: drop kevin's oversized layer
    entirely and instead amplify v3's grain-scale layers (more blobs, same
    tiny radius, same hue) so the far-distance hue-bin coverage kevin's
    docstring was chasing is preserved without ever drawing a blob big
    enough to read as a disk.

Per tile-zoom-check-procedural: verified 2x2-tiled zoom on both, no seam/
periodicity introduced (masked_speckle_fade/scatter_blobs are per-texel RNG
or wrap with % SIZE). Per guard-generator-output-procedural-asset: writes to
SCRATCH only; promote() is a separate explicit step run after visual review.
Never touches _pixel_blocks_gen64*.py or any live asset directly.
"""

import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _pixel_blocks_gen64 as base  # noqa: E402
import _pixel_blocks_gen64_v3_palette as v3  # noqa: E402
import _monanisa_logtop_clayplaster_fix as fix1  # noqa: E402

SIZE = base.SIZE
scatter_blobs = v3.scatter_blobs
paint_blobs = v3.paint_blobs
masked_speckle_fade = fix1.masked_speckle_fade

SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_floorboards_redsand_fix"
SCRATCH.mkdir(parents=True, exist_ok=True)
_LIVE_DIR = (base.ROOT / "assets" / "textures" / "blocks").resolve()
if SCRATCH.resolve() == _LIVE_DIR or _LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")

TILES = ("floorboards", "red_sand")


def save_scratch(img, name, suffix=""):
    p = SCRATCH / f"{name}{suffix}.png"
    img.save(p)
    return p


# ------------------------------------------------------------- floorboards
def make_floorboards_fixed():
    albedo, height, rough = base.make_floorboards()
    v3.polish_floorboards(albedo, height, rough)  # unchanged: small algae + nail heads

    board_h = 16
    seam_top = board_h - 2  # matches base.make_floorboards' `within >= board_h - 2`

    def make_fade(reach_lo, reach_hi, seed):
        col_rng = random.Random(seed)
        reach = {}

        def fade_fn(x, y):
            board = int(y) // board_h
            within = int(y) % board_h
            key = (board, int(x))
            if key not in reach:
                reach[key] = col_rng.uniform(reach_lo, reach_hi)
            r = reach[key]
            top = seam_top - r
            if within < top:
                return 0.0
            span = seam_top - top
            return min(1.0, (within - top) / max(span, 1e-6))
        return fade_fn

    moss_fade = make_fade(1.5, 4.0, "floorboards-moss-reach")
    moss_rng = random.Random("floorboards-moss-masked")
    masked_speckle_fade(moss_rng, albedo, height, rough, moss_fade,
                         h=98, s_range=(0.42, 0.55), v=0.42,
                         target_height=0.60, target_rough=0.84,
                         peak_density=0.40, blend_range=(0.35, 0.65), h_jitter=6)

    rust_fade = make_fade(1.0, 3.2, "floorboards-rust-reach")
    rust_rng = random.Random("floorboards-rust-masked")
    masked_speckle_fade(rust_rng, albedo, height, rough, rust_fade,
                         h=352, s_range=(0.52, 0.64), v=0.60,
                         target_height=0.46, target_rough=0.66,
                         peak_density=0.35, blend_range=(0.30, 0.55), h_jitter=6)

    # small mineral flecks -- already correct grain scale, kept verbatim
    r = random.Random("floorboards-v4")
    blobs = scatter_blobs(r, 6, (3, 5))
    paint_blobs(r, albedo, height, rough, blobs, h=208, s_range=(0.05, 0.12), v=0.42,
                target_height=0.42, target_rough=0.68, max_blend=1.0, h_jitter=6)
    return albedo, height, rough


# ---------------------------------------------------------------- red_sand
def make_red_sand_fixed():
    albedo, height, rough = base.make_red_sand()
    v3.polish_red_sand(albedo, height, rough)  # unchanged: correct-scale grain

    # Amplify the SAME grain-scale layers (more blobs, same tiny radius, same
    # hue) instead of kevin_v4's oversized disks -- preserves far-distance
    # hue-bin coverage without ever drawing a blob big enough to read as one.
    r = random.Random("red_sand-lock")
    mineral = scatter_blobs(r, 10, (0.7, 1.4))
    paint_blobs(r, albedo, height, rough, mineral, h=15, s_range=(0.10, 0.20), v=0.20,
                target_height=0.34, target_rough=0.70, max_blend=0.9, h_jitter=6)
    olivine = scatter_blobs(r, 8, (0.9, 1.7))
    paint_blobs(r, albedo, height, rough, olivine, h=110, s_range=(0.30, 0.42), v=0.38,
                target_height=0.56, target_rough=0.72, max_blend=0.85, h_jitter=8)
    shell = scatter_blobs(r, 5, (0.9, 1.5))
    paint_blobs(r, albedo, height, rough, shell, h=25, s_range=(0.08, 0.16), v=0.86,
                target_height=0.60, target_rough=0.52, max_blend=0.8, h_jitter=6)
    return albedo, height, rough


MAKE = {
    "floorboards": make_floorboards_fixed,
    "red_sand": make_red_sand_fixed,
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
    visual review. Usage: --promote floorboards | red_sand | all"""
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
