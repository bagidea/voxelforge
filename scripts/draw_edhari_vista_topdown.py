#!/usr/bin/env python3
"""Render an annotated top-down vista composition of maps/edhari.json.

Outputs _edhari_vista_topdown.png in the project root with:
- spawn marker (red)
- Sentinel Spire highlight (gold)
- foreground frame markers (blue squares)
- depth-layer boundaries (cyan lines)
- sight-line from spawn to the dominant landmark (orange dashed line)
"""
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

MAP = Path(__file__).resolve().parent.parent / "maps" / "edhari.json"
OUT = Path(__file__).resolve().parent.parent / "_edhari_vista_topdown.png"

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
        base = PALETTE.get(name, np.array([0x1a, 0x1a, 0x1a], dtype=np.uint8)).copy()
        lift = min(height[z, x] * 8, 80)
        base = np.clip(base.astype(np.int16) + lift, 0, 255).astype(np.uint8)
        img_arr[z, x] = base

scale = 12
h, w = img_arr.shape[:2]
img = Image.fromarray(img_arr).resize((w * scale, h * scale), Image.NEAREST)

legend_h = 100
left_pad = 28
out_img = Image.new("RGB", (img.width + left_pad, img.height + legend_h), (0x1a, 0x1a, 0x1a))
out_img.paste(img, (left_pad, legend_h))
draw = ImageDraw.Draw(out_img)

try:
    font = ImageFont.truetype("arial.ttf", 14)
    small = ImageFont.truetype("arial.ttf", 12)
except Exception:
    font = ImageFont.load_default()
    small = font

# Spawn / landmark positions
SPAWN = (32, 32)
SPIRE = (48, 4)

# Layer boundaries (matching verify_edhari_village.py composition gates)
FG_Z0, FG_Z1 = 24, 32
MG_Z0, MG_Z1 = 10, 23
BG_Z0, BG_Z1 = 0, 9

def world_to_img(wx, wz):
    return (left_pad + wx * scale, legend_h + wz * scale)

def draw_world_line(x0, z0, x1, z1, fill, width=2, dash=0):
    p0 = world_to_img(x0, z0)
    p1 = world_to_img(x1, z1)
    if dash:
        draw.line([p0, p1], fill=fill, width=width)
    else:
        draw.line([p0, p1], fill=fill, width=width)

def draw_world_rect(x0, z0, x1, z1, outline, width=2):
    p0 = world_to_img(x0, z0)
    p1 = world_to_img(x1, z1)
    draw.rectangle([p0, p1], outline=outline, width=width)

def draw_world_dot(wx, wz, fill, r=4):
    cx, cy = world_to_img(wx, wz)
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=fill)

# Layer boundary lines across the view cone
for z in (FG_Z1 + 0.5, MG_Z1 + 0.5):
    z_pixel = int(legend_h + z * scale)
    draw.line([(left_pad, z_pixel), (left_pad + W * scale, z_pixel)], fill=(0x00, 0xff, 0xff), width=2)

# Foreground frame pillars (vista composition)
for fx, fz in [(23, 26), (23, 27), (41, 26), (41, 27)]:
    draw_world_rect(fx - 0.4, fz - 0.4, fx + 0.4, fz + 0.4, outline=(0x00, 0x00, 0xff), width=2)

# Hero framing gateway: the spawn shelter now acts as a deliberate doorway.
# Magenta rectangle = the frame; cyan dot = camera position for the pulled shot.
draw_world_rect(29, 29, 35, 35, outline=(0xff, 0x00, 0xff), width=2)
draw_world_dot(32.5, 42, fill=(0x00, 0xff, 0xff), r=4)
draw_world_line(32.5, 42, 32.5, 20, fill=(0x00, 0xff, 0xff), width=2, dash=1)

# Sight-line from spawn to spire (dashed approximation)
steps = 24
for i in range(steps):
    if i % 2 == 0:
        t0 = i / steps
        t1 = (i + 1) / steps
        x0 = SPAWN[0] + (SPIRE[0] - SPAWN[0]) * t0
        z0 = SPAWN[1] + (SPIRE[1] - SPAWN[1]) * t0
        x1 = SPAWN[0] + (SPIRE[0] - SPAWN[0]) * t1
        z1 = SPAWN[1] + (SPIRE[1] - SPAWN[1]) * t1
        draw_world_line(x0, z0, x1, z1, fill=(0xff, 0x80, 0x00), width=2)

# Spawn and landmark markers
draw_world_dot(*SPAWN, fill=(0xff, 0x00, 0x00), r=5)
draw_world_dot(*SPIRE, fill=(0xff, 0xd7, 0x00), r=5)

# Title and legend
draw.text((10, 8), "Edhari vista composition (top-down)", fill=(0xff, 0xff, 0xff), font=font)
draw.text((10, 28), "red = spawn  |  gold = Sentinel Spire  |  blue squares = foreground frame  |  cyan = layer boundaries",
          fill=(0xcc, 0xcc, 0xcc), font=small)
draw.text((10, 46), "orange dashed = spawn->landmark sight-line  |  magenta box = hero framing gateway  |  cyan dot = camera",
          fill=(0xcc, 0xcc, 0xcc), font=small)

# Axis labels
for i in range(0, W + 1, 8):
    draw.text((left_pad + i * scale + 2, legend_h - 14), str(i), fill=(0x99, 0x99, 0x99), font=small)
    label = str(i)
    try:
        bbox = draw.textbbox((0, 0), label, font=small)
        text_w = bbox[2] - bbox[0]
    except Exception:
        text_w = 7 * len(label)
    draw.text((left_pad - text_w - 4, legend_h + i * scale + 2), label,
              fill=(0x99, 0x99, 0x99), font=small)

# Region labels
fg_mid = (FG_Z0 + FG_Z1) / 2
mg_mid = (MG_Z0 + MG_Z1) / 2
bg_mid = (BG_Z0 + BG_Z1) / 2
draw.text(world_to_img(2, fg_mid), "foreground", fill=(0x00, 0xff, 0xff), font=small)
draw.text(world_to_img(2, mg_mid), "midground", fill=(0x00, 0xff, 0xff), font=small)
draw.text(world_to_img(48, bg_mid), "background\nlandmark", fill=(0xff, 0xff, 0xff), font=small)

OUT.parent.mkdir(parents=True, exist_ok=True)
out_img.save(OUT)
print(f"saved {OUT}")
