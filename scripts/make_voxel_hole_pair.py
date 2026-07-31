#!/usr/bin/env python3
"""Build the before/after evidence pair for docs/voxel-hole-findings.md.

Left  = docs/assets/wide-hero-final.png       (pre-fix golden -- voxel missing)
Right = docs/assets/voxelfix/after-fix.png    (post-fix -- same staging, voxel back)
The red box is the world cell (5,4,2) projected through the locked wide camera
by scripts/voxel_hole_proof.py -- it is where the arithmetic says the collision
is, drawn on both frames so the eye checks the same pixels the maths named.
"""
from PIL import Image, ImageDraw, ImageFont

CROP = (600, 170, 1010, 530)          # the island + tumbler, both frames
HOLE = (752, 293, 843, 382)           # measured hole == projected cell (5,4,2)
PAIRS = [("BEFORE  2 cubes, 1 cell   teal 0.079",
          "docs/assets/wide-hero-final.png"),
         ("AFTER   VoxelGrid        teal 0.757 = web",
          "docs/assets/voxelfix/after-fix.png")]
OUT = "docs/assets/voxelfix/voxel-hole-before-after.png"

try:
    font = ImageFont.truetype("C:/Windows/Fonts/consolab.ttf", 15)
except OSError:
    font = ImageFont.load_default()

panels = []
for label, path in PAIRS:
    im = Image.open(path).convert("RGB")
    d = ImageDraw.Draw(im)
    d.rectangle(HOLE, outline=(255, 40, 40), width=3)
    im = im.crop(CROP)
    w, h = im.size
    out = Image.new("RGB", (w, h + 26), (18, 18, 18))
    out.paste(im, (0, 26))
    ImageDraw.Draw(out).text((6, 5), label, fill=(235, 235, 235), font=font)
    panels.append(out)

w, h = panels[0].size
sheet = Image.new("RGB", (w * 2 + 10, h), (18, 18, 18))
for i, p in enumerate(panels):
    sheet.paste(p, (i * (w + 10), 0))
sheet.save(OUT)
print(f"OK {OUT} {sheet.size}")
