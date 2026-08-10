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
    "grass":       np.array([0x5c, 0x8f, 0x4a], dtype=np.uint8),
    "dirt":        np.array([0x6b, 0x4c, 0x2a], dtype=np.uint8),
    "stone":       np.array([0x8a, 0x8a, 0x8a], dtype=np.uint8),
    "sand":        np.array([0xe0, 0xd2, 0xa0], dtype=np.uint8),
    "wood":        np.array([0x9c, 0x6b, 0x3a], dtype=np.uint8),
    "leaves":      np.array([0x3a, 0x74, 0x36], dtype=np.uint8),
    "snow":        np.array([0xf0, 0xec, 0xe0], dtype=np.uint8),
    "red_sand":    np.array([0xc8, 0x82, 0x46], dtype=np.uint8),
    "clay":        np.array([0x7e, 0x96, 0xa0], dtype=np.uint8),
    "gravel":      np.array([0x6e, 0x64, 0x5e], dtype=np.uint8),
    "cobblestone": np.array([0x8c, 0x8a, 0x78], dtype=np.uint8),
    "obsidian":    np.array([0x1a, 0x16, 0x20], dtype=np.uint8),
    "brick":       np.array([0x96, 0x5a, 0x3c], dtype=np.uint8),
    "moss":        np.array([0x4b, 0x6e, 0x37], dtype=np.uint8),
    "limestone":   np.array([0xde, 0xcc, 0xa8], dtype=np.uint8),
    "lamp":        np.array([0xf0, 0xb4, 0x50], dtype=np.uint8),
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
left_pad = 24  # room for two-digit z-axis labels on the left edge
out_img = Image.new("RGB", (img.width + left_pad, img.height + legend_h), (0x1a, 0x1a, 0x1a))
out_img.paste(img, (left_pad, legend_h))
draw = ImageDraw.Draw(out_img)

try:
    font = ImageFont.truetype("arial.ttf", 14)
    small = ImageFont.truetype("arial.ttf", 12)
except Exception:
    font = ImageFont.load_default()
    small = font

# Title + key info
draw.text((10, 8), "Edhari Village — top-down layout", fill=(0xff, 0xff, 0xff), font=font)
block_count = len(d["blocks"])
draw.text((10, 28), f"{block_count} blocks  |  spawn(32,32)  |  fire(32,29)  |  husk(32,25)  |  gate(32,3-5)",
          fill=(0xcc, 0xcc, 0xcc), font=small)

# Colour key
key_x = 10
key_y = 50
key_items = [
    ("grass", PALETTE["grass"]),
    ("dirt", PALETTE["dirt"]),
    ("stone", PALETTE["stone"]),
    ("sand", PALETTE["sand"]),
    ("wood", PALETTE["wood"]),
    ("moss", PALETTE["moss"]),
    ("lamp", PALETTE["lamp"]),
]
for label, col in key_items:
    draw.rectangle([key_x, key_y, key_x + 14, key_y + 14], fill=tuple(int(c) for c in col))
    draw.text((key_x + 20, key_y), label, fill=(0xcc, 0xcc, 0xcc), font=small)
    key_x += 78

# Axis labels every 8 blocks (now every 8*scale pixels in image space)
for i in range(0, W + 1, 8):
    draw.text((left_pad + i * scale + 2, legend_h - 14), str(i), fill=(0x99, 0x99, 0x99), font=small)
    # Right-align z labels inside the left padding so "16"/"24" are not clipped.
    label = str(i)
    try:
        bbox = draw.textbbox((0, 0), label, font=small)
        text_w = bbox[2] - bbox[0]
    except Exception:
        text_w = 7 * len(label)  # rough fallback for default font
    draw.text((left_pad - text_w - 4, legend_h + i * scale + 2), label,
              fill=(0x99, 0x99, 0x99), font=small)

OUT.parent.mkdir(parents=True, exist_ok=True)
out_img.save(OUT)
print(f"saved {OUT}")
