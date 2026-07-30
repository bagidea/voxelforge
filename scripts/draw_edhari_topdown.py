#!/usr/bin/env python3
"""Render a top-down view of maps/edhari.json.

Shows the highest solid block in each (x,z) column, coloured by block type and
shaded by height so landmarks, paths, and elevation changes read clearly.
"""
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

MAP = Path(__file__).resolve().parent.parent / "maps" / "edhari.json"
OUT = Path(__file__).resolve().parent.parent / "docs" / "assets" / "edhari-village-topdown.png"

PALETTE = {
    "grass": np.array([0x5c, 0x8f, 0x4a], dtype=np.uint8),
    "dirt":  np.array([0x6b, 0x4c, 0x2a], dtype=np.uint8),
    "stone": np.array([0x8a, 0x8a, 0x8a], dtype=np.uint8),
    "sand":  np.array([0xe0, 0xd2, 0xa0], dtype=np.uint8),
}

with open(MAP, encoding="utf-8") as f:
    d = json.load(f)

W = D = 64
height = np.zeros((D, W), dtype=np.int32)
block = np.full((D, W), "air", dtype=object)
for b in d["blocks"]:
    x, y, z, name = b["x"], b["y"], b["z"], b["block"].lower()
    if y > height[z, x]:
        height[z, x] = y
        block[z, x] = name

img_arr = np.zeros((D, W, 3), dtype=np.uint8)
for z in range(D):
    for x in range(W):
        name = block[z, x]
        base = PALETTE.get(name, np.array([0, 0, 0], dtype=np.uint8)).copy()
        # Lighten with height so raised terraces/landmarks stand out.
        lift = min(height[z, x] * 8, 80)
        base = np.clip(base.astype(np.int16) + lift, 0, 255).astype(np.uint8)
        img_arr[z, x] = base

# Upscale for readability (4x -> 256x256 base image, then add legend padding)
scale = 8
h, w = img_arr.shape[:2]
img = Image.fromarray(img_arr).resize((w * scale, h * scale), Image.NEAREST)

# Add a legend and labels
legend_h = 90
out_img = Image.new("RGB", (img.width, img.height + legend_h), (0x1a, 0x1a, 0x1a))
out_img.paste(img, (0, legend_h))
draw = ImageDraw.Draw(out_img)

try:
    font = ImageFont.truetype("arial.ttf", 14)
    small = ImageFont.truetype("arial.ttf", 12)
except Exception:
    font = ImageFont.load_default()
    small = font

# Title + key info
draw.text((10, 8), "Edhari Village — top-down layout", fill=(0xff, 0xff, 0xff), font=font)
draw.text((10, 28), f"6483 blocks  |  spawn(32,32)  |  fire(32,29)  |  gate(32,3-5)",
          fill=(0xcc, 0xcc, 0xcc), font=small)

# Colour key
key_x = 10
key_y = 50
for label, col in [("grass", PALETTE["grass"]), ("dirt", PALETTE["dirt"]),
                   ("stone path/walls", PALETTE["stone"]), ("sand sigil", PALETTE["sand"])]:
    draw.rectangle([key_x, key_y, key_x + 14, key_y + 14], fill=tuple(int(c) for c in col))
    draw.text((key_x + 20, key_y), label, fill=(0xcc, 0xcc, 0xcc), font=small)
    key_x += 110

# Axis labels every 8 blocks (now every 8*scale pixels in image space)
for i in range(0, W + 1, 8):
    draw.text((i * scale + 2, legend_h - 14), str(i), fill=(0x99, 0x99, 0x99), font=small)
    draw.text((2, legend_h + i * scale + 2), str(i), fill=(0x99, 0x99, 0x99), font=small)

OUT.parent.mkdir(parents=True, exist_ok=True)
out_img.save(OUT)
print(f"saved {OUT}")
