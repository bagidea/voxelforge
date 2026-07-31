"""Crop the two Steam key-art masters into the capsule set. Run once, ad hoc — not part of the build.

Sizes below are Valve's CURRENT store-asset spec (verified against
https://partner.steamgames.com/doc/store/assets/standard and
https://partner.steamgames.com/doc/store/assets/libraryassets on 2026-08-01), not the pre-August-2024
sizes this script used to produce. See docs/steam-store-art.md §6 for the correction note and
docs/steam-art-review-2026-08-01.md (Flamingo) for the review that first caught this.

Header capsule and main capsule keep the exact aspect ratio Valve used before August 2024 — the new
sizes are precisely 2x the old ones (920x430 = 2x460x215, 1232x706 = 2x616x353) — so the same crop
bands apply; only the output resolution changed. Small capsule, vertical capsule, and page background
are newly-added assets with their own aspect ratios and are cropped fresh.

This pass fixes DIMENSIONS ONLY. It does not address the composition/palette/safe-area/logo findings
in docs/steam-art-review-2026-08-01.md (F2 no logo, F4 hero safe-area, F6/F8 crop amputates the
character, F9/F10 palette+shadow law) — those need new key art or a compositing pass, not a resize,
and are a separate decision for Director (see that review's §4 options A/B/C).
"""
from PIL import Image

SRC_DIR = r"E:\Projects\bagidea-ai-agents-office\workspace\uploads"
OUT_DIR = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\steam"

landscape = Image.open(f"{SRC_DIR}\\gen_1785505599301.png").convert("RGB")
portrait = Image.open(f"{SRC_DIR}\\gen_1785505667008.png").convert("RGB")

landscape.save(f"{OUT_DIR}\\key-art-landscape-master.png")
portrait.save(f"{OUT_DIR}\\key-art-portrait-master.png")

W, H = landscape.size  # 1024x1024


def crop_band(img, top, bottom, target_w, target_h, name):
    w, h = img.size
    band = img.crop((0, top, w, bottom))
    band = band.resize((target_w, target_h), Image.LANCZOS)
    band.save(f"{OUT_DIR}\\{name}")
    print(name, band.size)


def crop_vertical(img, target_w, target_h, name):
    """Full-height centered crop, for portrait-derived assets (mirrors the library-capsule logic)."""
    w, h = img.size
    band_w = int(h * (target_w / target_h))
    x0 = (w - band_w) // 2
    band = img.crop((x0, 0, x0 + band_w, h)).resize((target_w, target_h), Image.LANCZOS)
    band.save(f"{OUT_DIR}\\{name}")
    print(name, band.size)


# ---- Store capsules (Valve spec, current as of Aug 2024 — the ones this fix is about) ----

# Header capsule 920x430 (~2.14:1, unchanged ratio from the pre-2024 460x215 slot, just 2x resolution)
crop_band(landscape, 140, 619, 920, 430, "header-capsule-920x430.png")

# Main capsule 1232x706 (~1.75:1, unchanged ratio from the pre-2024 616x353 slot, just 2x resolution —
# this is a mild ~1.2x upscale past the 1024-wide master since 1232 > 1024)
crop_band(landscape, 130, 717, 1232, 706, "main-capsule-1232x706.png")

# Small capsule 462x174 (~2.66:1) — NEW asset, not in the old set at all. Same vertical composition
# center as header/main (this crop, like those, inherits Flamingo's F8 "amputates the character"
# finding — that's a composition fix for the master re-shoot, not something a resize can solve).
crop_band(landscape, 187, 573, 462, 174, "small-capsule-462x174.png")

# Vertical capsule 748x896 (~0.83:1) — NEW asset. Full-height crop of the portrait master, same
# approach as the library capsule below but a different target aspect.
crop_vertical(portrait, 748, 896, "vertical-capsule-748x896.png")

# Page Background 1438x810 (~1.78:1, optional per spec but requested) — NEW asset. A ~1.4x upscale
# past the 1024-wide master; much milder than the library hero's 3.75x, but still not a native render.
crop_band(landscape, 135, 712, 1438, 810, "page-background-1438x810.png")

# ---- Library assets (unchanged — still Valve's current spec, not affected by the Aug 2024 change) ----

# Library hero — ultrawide band, upscaled straight to the shipped 3840x1240 in one Lanczos
# pass (the crop band itself is ~1024x330 since 3840px wide exceeds this master's native
# 1024x1024 resolution — this IS the upscale step, not a placeholder; re-run this file if it
# ever ships soft at full zoom, this time rendering the master at native 3840px instead)
crop_band(landscape, 330, 660, 3840, 1240, "library-hero-3840x1240.png")

# Library capsule 600x900 (2:3 portrait) — full height, centered crop on the portrait master
crop_vertical(portrait, 600, 900, "library-capsule-600x900.png")
