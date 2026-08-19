#!/usr/bin/env python3
"""Repaint grass_side / grass_top / leaves — same diagonal sine-gradient
"candy-stripe" defect as roof_tile.png (fixed 2026-08-19), all four written by
scripts/_kevin_palette_push.py's `periodic()` helper: a sum of only 3-5
integer-frequency sines is dominated by its lowest term, which reads as a
hard diagonal gradient once tiled, not organic ground cover.

Fix: replace periodic() with isotropic blurred value-noise (wrap-tiled, no
preferred direction -> no stripes), keep the extreme green G/R the palette
push needed (survive the warm sunset light, see commit 5ebe745) and the
blue-flower / yellow-green accents, but as small round clusters instead of
long diagonal streaks.

Touches albedo .png only. 64x64, seamless wrap. No Rust, no rebuild.
"""
from __future__ import annotations

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

S = 64
OUT = "assets/textures/blocks"


def value_noise(seed: int, blur: float) -> np.ndarray:
    """Isotropic blurred white noise, wrap-tiled -> seamless, no direction bias."""
    rng = np.random.default_rng(seed)
    v = rng.uniform(0, 1, (S, S))
    v = gaussian_filter(v, sigma=blur, mode="wrap")
    v -= v.min()
    return v / (v.max() + 1e-9)


def blobs(seed: int, blur: float, coverage: float) -> np.ndarray:
    """Sparse round clusters via high-percentile threshold on blurred noise."""
    n = value_noise(seed, blur)
    thresh = np.quantile(n, 1 - coverage)
    return (n > thresh).astype(np.float64)


def blend(base: np.ndarray, accent: np.ndarray, mask: np.ndarray) -> np.ndarray:
    m = mask[..., None]
    return base * (1 - m) + accent[None, None, :] * m


def save(name: str, rgb: np.ndarray):
    rgb = np.clip(rgb, 0, 255).astype(np.uint8)
    Image.fromarray(rgb, "RGB").save(f"{OUT}/{name}.png")
    print(f"wrote {OUT}/{name}.png")


base_green = np.array([16, 168, 92], dtype=np.float64)   # G/R ~10.5, stays green under warm light
yellowgreen = np.array([112, 196, 70], dtype=np.float64)
flower = np.array([235, 222, 96], dtype=np.float64)       # buttery yellow (daisy/dandelion), B<G<R always
dirt = np.array([96, 66, 40], dtype=np.float64)
dirt_dark = np.array([72, 48, 28], dtype=np.float64)

# ---- grass_top : fine blade mottling + meadow patches + flower dots --------
fine = value_noise(101, blur=1.1)                       # small-scale blade texture
top = base_green[None, None, :] * (0.78 + 0.4 * fine[..., None])
meadow_mask = blobs(202, blur=3.2, coverage=0.22)        # large soft patches, round not streaky
top = blend(top, yellowgreen, meadow_mask)
flower_mask = blobs(303, blur=0.8, coverage=0.03)        # small round dots, sparse
top = blend(top, flower, flower_mask)
save("grass_top", top)

# ---- grass_side : turf strip over dirt, straight-ish (jagged) seam ---------
fine2 = value_noise(111, blur=1.1)
side = base_green[None, None, :] * (0.75 + 0.45 * fine2[..., None])
side = blend(side, yellowgreen, blobs(203, blur=2.6, coverage=0.16))

seam = value_noise(112, blur=1.4)                         # jitter the grass/dirt border
row = np.arange(S)[:, None]
border = S // 3 + (seam[:, 0:1] * 6 - 3).astype(np.int64)  # +-3px jagged, not diagonal
dirt_mask = (row >= border).astype(np.float64)
dirt_tex = dirt[None, None, :] * (0.8 + 0.4 * value_noise(113, blur=0.9)[..., None])
dirt_tex = np.where((value_noise(114, blur=0.7) > 0.7)[..., None], dirt_dark[None, None, :], dirt_tex)
side = side * (1 - dirt_mask[..., None]) + dirt_tex * dirt_mask[..., None]
save("grass_side", side)

# ---- leaves : two-tone canopy mottling + light-catch highlights ------------
lo3 = value_noise(121, blur=2.1)                          # bumped from 1.6: blade clumps read as foliage, not grain
dark_leaf = np.array([12, 108, 54], dtype=np.float64)
bright_leaf = np.array([26, 188, 98], dtype=np.float64)
leaves_rgb = dark_leaf[None, None, :] * (1 - lo3[..., None]) + bright_leaf[None, None, :] * lo3[..., None]
leaves_rgb = blend(leaves_rgb, np.array([150, 205, 40], np.float64), blobs(204, blur=1.3, coverage=0.10))
save("leaves", leaves_rgb)

print("done")
