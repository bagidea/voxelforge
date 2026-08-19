#!/usr/bin/env python3
"""_kevin_palette_push.py — one-off palette-breadth push on the 5 dominant
materials of the beach_dusk graded plate (hue_bins 6 -> target 9+).

WHY
---
The graded frame `_pixel_world_AFTER.png` measures hue_bins=6, all in the warm
0-69 deg band (bins 0-4 + 6). The v3 polish added 1px magenta/berry accents per
tile; at the plate's render distance those mip-crush to nothing AND the warm
sunset light (inferred ~0.63 G / 0.39 B relative to R from the sand albedo ->
render pair) collapses every albedo hue toward orange.

The fix that survives BOTH effects: make the base hues EXTREME in the channel
the light attenuates (green needs G/R > ~1.6 to stay green; blue needs B/R >
~2.5 to stay blue) and make accents LARGE (multi-px patches, not single pixels)
so they survive the ~2-3x minification.

Touches albedo .png only. 64x64, seamless (integer-frequency periodic noise).
No Rust, no rebuild — the exe re-reads assets/textures/blocks at startup.
"""

from __future__ import annotations

import numpy as np
from PIL import Image

S = 64
X, Y = np.meshgrid(np.arange(S, dtype=np.float64), np.arange(S, dtype=np.float64))
OUT = "assets/textures/blocks"


def periodic(seed: int, freqs, amps=None) -> np.ndarray:
    """Sum of integer-frequency sines -> seamless over a 64px period."""
    rng = np.random.default_rng(seed)
    v = np.zeros((S, S), dtype=np.float64)
    amps = amps or [1.0] * len(freqs)
    for (fx, fy), a in zip(freqs, amps):
        ph = rng.uniform(0, 2 * np.pi)
        v += a * np.sin(2 * np.pi * (X * fx + Y * fy) / S + ph)
    v -= v.min()
    return v / (v.max() + 1e-9)  # [0,1]


def grain(seed: int, strength: float = 0.06) -> np.ndarray:
    rng = np.random.default_rng(seed)
    return rng.uniform(-strength, strength, (S, S))


def blend(base: np.ndarray, accent: np.ndarray, mask: np.ndarray) -> np.ndarray:
    m = mask[..., None]
    return base * (1 - m) + accent * m


def save(name: str, rgb: np.ndarray):
    rgb = np.clip(rgb, 0, 255).astype(np.uint8)
    Image.fromarray(rgb, "RGB").save(f"{OUT}/{name}.png")
    print(f"wrote {OUT}/{name}.png")


# ---- grass_top : extreme green base + yellow-green patches + blue flowers ---
# base green G/R ~ 5.5 -> survives warm light as green (bin 7-9)
lo = periodic(101, [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2)])
base_green = np.array([14, 170, 95], dtype=np.float64)   # cooler -> pushes bin 8-9
dark_green = np.array([10, 100, 70], dtype=np.float64)
base = base_green[None, None, :] * (0.75 + 0.5 * lo[..., None])  # shade variation

# yellow-green meadow patches (bin 5-6), large + smooth
meadow = periodic(202, [(1, 1), (1, 2)], amps=[1.0, 0.6])
meadow_mask = (meadow > 0.74).astype(np.float64)
yellowgreen = np.array([108, 198, 68], dtype=np.float64)
base = blend(base, yellowgreen[None, None, :], meadow_mask)

# blue wildflower clusters (bin 21-24), large so they survive minification
flowers = periodic(303, [(2, 1), (1, 2), (2, 2)])
flower_mask = ((flowers > 0.70).astype(np.float64)
               * (periodic(304, [(3, 2)]) > 0.30).astype(np.float64))
blue = np.array([40, 70, 230], dtype=np.float64)
base = blend(base, blue[None, None, :], flower_mask)

save("grass_top", base)

# ---- grass_side : green turf over dirt, same hue logic ---------------------
lo2 = periodic(111, [(1, 0), (0, 1), (1, 1)])
side = base_green[None, None, :] * (0.7 + 0.55 * lo2[..., None])
dirt = np.array([96, 66, 40], dtype=np.float64)
side[(S * 2) // 3:, :] = dirt[None, None, :] * (0.8 + 0.4 * periodic(112, [(1, 1)])[(S * 2) // 3:, :, None])
side = blend(side, yellowgreen[None, None, :], (periodic(203, [(1, 2)]) > 0.66).astype(np.float64))
save("grass_side", side)

# ---- leaves : bright + dark green with yellow-green highlights -------------
lo3 = periodic(121, [(1, 1), (2, 1), (1, 2), (2, 2)])
leaf = np.array([14, 165, 80], dtype=np.float64)
leaves_rgb = leaf[None, None, :] * (0.6 + 0.7 * lo3[..., None])
leaves_rgb = blend(leaves_rgb, np.array([150, 205, 40], np.float64)[None, None, :],
                   (periodic(204, [(1, 2)]) > 0.72).astype(np.float64))
save("leaves", leaves_rgb)

# ---- brick : true red terracotta + lighter mortar (no more grey) -----------
brick = np.array([182, 66, 42], dtype=np.float64)     # hue ~8, G/R 0.36 -> stays red
mortar = np.array([208, 148, 118], dtype=np.float64)  # warm light mortar
# running bond: mortar courses
course = periodic(131, [(0, 3)], amps=[1.0])
bricks_rgb = np.where((course > 0.55)[..., None], mortar[None, None, :], brick[None, None, :])
# vertical joints every 32px, offset per course
jy = (np.arange(S) // 16) % 2
jx = (np.arange(S)[None, :] + jy[:, None] * 16) % 32 == 0
bricks_rgb[jx] = mortar
bricks_rgb = bricks_rgb * (0.85 + 0.3 * periodic(132, [(1, 1)])[..., None])
save("brick", bricks_rgb)

# ---- roof_tile : red-brown with red variation (stays warm but distinct) ----
lo5 = periodic(141, [(1, 0), (0, 1), (1, 1)])
roof = np.array([158, 66, 44], dtype=np.float64)
roof_rgb = roof[None, None, :] * (0.7 + 0.6 * lo5[..., None])
roof_rgb = blend(roof_rgb, np.array([196, 84, 52], np.float64)[None, None, :],
                 (periodic(205, [(2, 1)]) > 0.6).astype(np.float64))
save("roof_tile", roof_rgb)

print("done")
