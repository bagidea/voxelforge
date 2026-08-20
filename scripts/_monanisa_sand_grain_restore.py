"""
Monanisa / art lane -- 2026-08-20 (2): sand grain-contrast restore.

Director confirmed the material-lock pass (`_monanisa_material_lock_pass.py`,
make_sand_v3) fixed every off-material hue on every tile, sand included --
no more teal/neon. But sand alone came out "jued" (bland): the lock pass
kept `base.make_sand()`'s per-texel grain loop UNCHANGED and only replaced
the old ~20-blob accent stack (which carried the teal/neon) with a much
smaller 15-blob one. Net effect: hue is correct now, but total visible grain
density/contrast dropped hard vs the old (buggy-hued) version, so up close
the tile reads as a near-flat wash instead of individual sand grains.

Fix, sand-only, hue family UNCHANGED (still H=30-42 warm neutral tan --
same range make_sand_v3 already locked in, no teal h150-190, no saturated
yellow):
  1. Amplify the base per-texel grain loop's contrast (jitter amplitude +
     shadow/crest depth + frequency) -- this is literally "grain": per-texel
     value variation, not blob accents. Hue itself untouched (still H=42
     base +/- jitter).
  2. Reuse make_sand_v3's three accent blob layers (shell / pebble / glint,
     all already locked to the sand hue family) verbatim, only turning up
     their count slightly so grain clusters read at both close range and at
     the 2x2 tile-zoom check.

Per guard-generator-output-procedural-asset: writes to SCRATCH only; live
promotion is a separate explicit step (promote()) run after visual review.
Per tile-zoom-check-procedural: blob placement already wraps with `% SIZE`
(paint_blobs), and the base loop is per-texel RNG (no sine-sum), so this
carries no periodicity risk -- still rendered 2x2 to confirm seams.
"""

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
scatter_blobs = v3.scatter_blobs
paint_blobs = v3.paint_blobs

SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_sand_grain_restore"
SCRATCH.mkdir(parents=True, exist_ok=True)
_LIVE_DIR = (base.ROOT / "assets" / "textures" / "blocks").resolve()
if SCRATCH.resolve() == _LIVE_DIR or _LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")


def save_scratch(img, name, suffix=""):
    p = SCRATCH / f"{name}{suffix}.png"
    img.save(p)
    return p


def make_sand_base_grain():
    """Same shape as base.make_sand(), contrast amplified. Hue target
    UNCHANGED (H=42, S=0.32, V=0.86) -- only the per-texel variance grows."""
    albedo, height, rough = base.new_state()
    rng = random.Random("sand")  # same seed as base.make_sand -- reproducible
    H, S, V = 42, 0.32, 0.86
    BR = 0.93
    for y in range(SIZE):
        for x in range(SIZE):
            v_ref = V
            v = V + jitter(rng, 0, 0.065)          # was 0.04
            s = S + jitter(rng, 0, 0.06)            # was 0.05
            rr = BR
            g = rng.random()
            if g < 0.22:                            # was 0.10
                v -= 0.16                           # was -0.10
                rr += 0.05                           # was +0.03
            elif g < 0.32:                          # was 0.14 (band width 0.04 -> 0.10)
                v += 0.13                           # was +0.08
                rr -= 0.08                           # was -0.05
            emit(albedo, height, rough, x, y, H + jitter(rng, 0, 4), s, v, v_ref, rr)
    return albedo, height, rough


def make_sand_v4():
    albedo, height, rough = make_sand_base_grain()
    r = random.Random("sand-lock")  # same seeds as make_sand_v3 -- same blob layout, just more of them

    shells = scatter_blobs(r, 9, (0.9, 1.5))          # was 6
    paint_blobs(r, albedo, height, rough, shells, h=24, s_range=(0.05, 0.12), v=0.90,
                target_height=0.60, target_rough=0.55, max_blend=0.8, h_jitter=8)

    pebbles = scatter_blobs(r, 9, (0.7, 1.3))          # was 6
    paint_blobs(r, albedo, height, rough, pebbles, h=30, s_range=(0.08, 0.16), v=0.30,
                target_height=0.36, target_rough=0.68, max_blend=0.85, h_jitter=6)

    glints = scatter_blobs(r, 4, (1.5, 2.5))           # was 3
    paint_blobs(r, albedo, height, rough, glints, h=40, s_range=(0.22, 0.32), v=0.72,
                target_height=0.50, target_rough=0.40, max_blend=0.7, h_jitter=5)
    return albedo, height, rough


def main():
    albedo, height, rough = make_sand_v4()
    assert albedo.size == (SIZE, SIZE)
    save_scratch(albedo, "sand")
    save_scratch(base.normal_from_height(height), "sand", "_n")
    save_scratch(base.roughness_img(rough), "sand", "_r")
    print(f"wrote sand (+ _n/_r) to {SCRATCH}")


def promote():
    """Explicit promotion step -- copies scratch sand.png/_n/_r over the
    live assets/textures/blocks/ triplet. Run only after visual review."""
    import shutil
    for suffix in ("", "_n", "_r"):
        src = SCRATCH / f"sand{suffix}.png"
        dst = _LIVE_DIR / f"sand{suffix}.png"
        shutil.copyfile(src, dst)
        print(f"promoted {src} -> {dst}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--promote":
        promote()
    else:
        main()
