"""Library-hero safe-area check against the Valve spec.

Valve Library Assets: the library hero's safe area is the CENTRE 860 x 380 band
(a horizontal strip in the middle of the 3840x1240 frame) -- not the full height.
Anything outside that band can be cropped as the client window is resized.
"""
import os
import numpy as np
from PIL import Image, ImageDraw

D = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\steam"
OUT = os.path.join(D, "review")

im = Image.open(os.path.join(D, "library-hero-3840x1240.png")).convert("RGB")
a = np.asarray(im).astype(np.float32)
L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
W, H = im.size

# Auren = the big dark foreground mass in the right-of-centre area.
dark = L < np.percentile(L, 20)
colmass = dark.mean(axis=0)
# the hero column band = the longest run where >45% of the column is dark
hot = colmass > 0.45
xs = np.where(hot)[0]
x0, x1 = int(xs.min()), int(xs.max())
rowmass = dark[:, x0:x1].mean(axis=1)
ys = np.where(rowmass > 0.45)[0]
y0, y1 = int(ys.min()), int(ys.max())

SAFE_W, SAFE_H = 860, 380
sx0, sx1 = (W - SAFE_W) // 2, (W + SAFE_W) // 2
sy0, sy1 = (H - SAFE_H) // 2, (H + SAFE_H) // 2
print(f"hero image        : {W}x{H}")
print(f"Valve safe area   : x {sx0}..{sx1}  y {sy0}..{sy1}  (centre {SAFE_W}x{SAFE_H} band)")
print(f"Auren dark mass   : x {x0}..{x1}  y {y0}..{y1}  (w={x1-x0}, h={y1-y0})")

inside_x = max(0, min(x1, sx1) - max(x0, sx0))
inside_y = max(0, min(y1, sy1) - max(y0, sy0))
print(f"Auren inside safe : horizontally {inside_x}px of {x1-x0}px = {100*inside_x/max(1,x1-x0):.1f}%")
print(f"                    vertically   {inside_y}px of {y1-y0}px = {100*inside_y/max(1,y1-y0):.1f}%")

# true 2-D overlap: fraction of the hero's dark pixels that land inside the safe band
mass = dark[y0:y1 + 1, x0:x1 + 1]
box = np.zeros_like(dark, dtype=bool)
box[sy0:sy1, sx0:sx1] = True
inbox = (dark & box).sum()
print(f"2-D coverage      : {inbox} of {dark[y0:y1+1, x0:x1+1].sum()} hero px inside band "
      f"= {100*inbox/max(1, mass.sum()):.1f}%")

print(f"top of hero mass  : y={y0} vs safe-area top y={sy0} -> "
      f"{'ABOVE SAFE AREA (croppable)' if y0 < sy0 else 'inside'}")
print(f"frame top edge    : {'CLIPPED BY FRAME' if y0 <= 2 else f'clear of frame by {y0}px'}")
print(f"right-edge        : hero mass ends at x={x1} of {W-1} -> {'TOUCHES RIGHT EDGE' if x1 >= W-3 else 'clear'}")

# head band = top 30% of the hero mass
hy1 = y0 + int(0.30 * (y1 - y0))
headcols = np.where(dark[y0:hy1, :].mean(axis=0) > 0.4)[0]
if len(headcols):
    hx0, hx1 = int(headcols.min()), int(headcols.max())
    hin = max(0, min(hx1, sx1) - max(hx0, sx0))
    hvin = max(0, min(hy1, sy1) - max(y0, sy0))
    print(f"HEAD/hood band    : x {hx0}..{hx1}  y {y0}..{hy1}")
    print(f"                    inside safe X = {100*hin/max(1,hx1-hx0):.1f}%  "
          f"inside safe Y = {100*hvin/max(1,hy1-y0):.1f}%  "
          f"-> {'FACE WILL CROP' if (hin/max(1,hx1-hx0) < 0.95 or hvin/max(1,hy1-y0) < 0.95) else 'ok'}")

ov = im.resize((1920, 620), Image.LANCZOS).convert("RGB")
d = ImageDraw.Draw(ov, "RGBA")
s = 1920 / W
d.rectangle([sx0 * s, sy0 * s, sx1 * s, sy1 * s],
            fill=(0, 255, 120, 40), outline=(0, 255, 140, 255), width=3)
d.rectangle([x0 * s, y0 * s, x1 * s, min(619, y1 * s)], outline=(255, 70, 70, 255), width=3)
d.text((sx0 * s + 8, sy0 * s + 8), "Valve safe area 860 x 380 (centre band)", fill=(180, 255, 200, 255))
d.text((x0 * s + 8, 8), "Auren (red) - outside the band on BOTH axes", fill=(255, 170, 170, 255))
p = os.path.join(OUT, "hero-safe-area-check.png")
ov.save(p)
print("wrote", p)
