"""
Monanisa / art lane -- 2026-08-21: PBR gamut-floor pass (mona/pbr lane).

MEASURED PROBLEM (Sun's brief): in all-orange/red scenes, 12 of 16 graded
plates have 98%+ of pixels sitting at literal B=0. Scope is material, not
light (Flamingo owns lighting) -- so the question is whether base-color /
metallic / roughness values are saturating against the gamut ceiling.

INVESTIGATION
--------------
1. metallic/roughness (client/src/voxel.rs::block_surface) are scalar,
   material-family constants (wood/stone/powder/foliage/glass/emitter) --
   none are colour-channel-specific and none are at 0/1 in a way that would
   crush a specific channel. Not the source.
2. Every live 64x64 base-colour PNG in assets/textures/blocks/ was read
   directly and histogrammed: NONE has any B=0 pixel today (0.00% across
   all 19). So the "98% B=0" plates are not literal texture clipping --
   that combination only appears after Flamingo's light/tonemap multiplies
   an already-thin channel down past the 8-bit rounding floor. Consistent
   with the scar this office already logged twice (gamut-clip-fakes-colour-
   gates, blue-gate-unsatisfiable-with-clip): a channel sitting a few percent
   above 0 reads as "not clipped" in a static per-texture histogram but
   clips the instant ANY multiplicative darkening (warm light, low exposure,
   tonemap shoulder) touches it.
3. What IS a genuine material-side number: per-texture minimum-channel
   headroom (worst single texel's weakest channel, as % of 8-bit range).
   Measured on the live art (post every prior structural fix -- material-
   lock pass 54d4d81, star-splat fix be365e9, floorboards/red_sand contain
   012ca11, all already baked into these PNGs):

     leaves 3.5%  grass_top 6.3%  oak_log_side 6.7%  dirt 7.1%
     grass_side 7.8%  brick 11.4%  roof_tile 11.8%  lamp 12.9%
     glass 14.5%  metal 14.9%  red_sand 15.7%  oak_planks 16.5%
     floorboards 21.6%  stone_bricks 22.0%  oak_log_top 23.9%
     sand 28.2%  snow 42.4%  water 46.7%  clay_plaster 43.5%

   No real dielectric material measures anywhere near 0 reflectance in any
   visible band -- a PBR base-colour validator (Substance/Marmoset-style)
   flags exactly this: a channel with under ~10-12% headroom is one exposure
   stop away from an unrecoverable clip, which is what happened here.

FIX
---
A single, well-defined, hue-preserving operation: HSV identity min_channel
= V*(1-S) holds for every hue, not just warm ones, so "pull S back until the
weakest channel clears a floor" is one formula regardless of material colour:

    if V*(1-S) < FLOOR:  S_new = 1 - FLOOR/V   (H, V unchanged)

Applied PER-PIXEL, only to texels that are actually below the floor -- most
of every tile is untouched (median well above the floor already), so this
does NOT re-author any material's look, hue family, or the structural fixes
already locked into these PNGs (masked-speckle containment, no floating
disks, family-correct accent hues). It only trims the saturation of the
handful of near-clip texels enough to survive one more stop of downstream
darkening. _n/_r maps are untouched (derived from height/roughness, not
albedo colour).

Per guard-generator-output-procedural-asset: operates on the ALREADY-LIVE
PNGs (not a from-scratch regen -- tracing four separate bespoke generator
chains, confirmed by direct comparison, would have silently reverted
_monanisa_material_lock_pass.py / _monanisa_brick_regrain.py / the
floorboards+red_sand fix). Writes to SCRATCH; promote() is explicit.
Per backup-procedural-asset-generator: the live PNGs were snapshotted to
assets/textures/blocks_pbr_lock_backup_20260821/ before this ran.
"""

import sys
from pathlib import Path

from PIL import Image
import colorsys

ROOT = Path(__file__).resolve().parent.parent
LIVE_DIR = (ROOT / "assets" / "textures" / "blocks").resolve()
SCRATCH = Path(__file__).resolve().parent / "_out" / "monanisa_pbr_gamut_floor"
SCRATCH.mkdir(parents=True, exist_ok=True)
if SCRATCH.resolve() == LIVE_DIR or LIVE_DIR in SCRATCH.resolve().parents:
    sys.exit(f"refusing to run: SCRATCH ({SCRATCH}) resolves inside the live asset folder")

FLOOR = 30.0 / 255.0  # ~11.8% of full range -- one stop of headroom

NAMES = [
    "oak_planks", "oak_log_side", "oak_log_top", "stone_bricks", "sand",
    "grass_top", "grass_side", "leaves", "roof_tile", "glass",
    "clay_plaster", "floorboards", "dirt", "brick", "lamp", "red_sand",
    "snow", "water", "metal",
]


def apply_gamut_floor(img):
    img = img.convert("RGBA")
    px = img.load()
    w, h = img.size
    changed = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            hh, s, v = colorsys.rgb_to_hsv(r / 255.0, g / 255.0, b / 255.0)
            if v <= 0.0:
                continue
            min_ch = v * (1.0 - s)
            if min_ch < FLOOR - 1e-6:
                s_new = 1.0 - FLOOR / v
                s_new = max(0.0, min(1.0, s_new))
                nr, ng, nb = colorsys.hsv_to_rgb(hh, s_new, v)
                px[x, y] = (round(nr * 255), round(ng * 255), round(nb * 255), a)
                changed += 1
    return img, changed


def main():
    names = sys.argv[1:] or NAMES
    print(f"{'name':<14} {'texels changed':>15} {'of':>4}")
    for name in names:
        src = LIVE_DIR / f"{name}.png"
        img = Image.open(src)
        out, changed = apply_gamut_floor(img)
        dst = SCRATCH / f"{name}.png"
        out.save(dst)
        total = img.size[0] * img.size[1]
        print(f"{name:<14} {changed:>15} / {total}")


def promote():
    import shutil
    which = sys.argv[2:] or NAMES
    for name in which:
        src = SCRATCH / f"{name}.png"
        dst = LIVE_DIR / f"{name}.png"
        shutil.copyfile(src, dst)
        print(f"promoted {src} -> {dst}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--promote":
        promote()
    else:
        main()
