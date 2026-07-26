#!/usr/bin/env python3
"""Eyedrop grader for the golden beauty-shot ref, per look-acceptance-rubric.md.
Samples fixed points + whole-frame stats so PASS/FAIL is measured, not eyeballed."""
import sys
from PIL import Image

img = Image.open(sys.argv[1] if len(sys.argv) > 1 else "docs/assets/golden-beauty-shot-ref.png").convert("RGB")
W, H = img.size
px = img.load()

def L(r, g, b):
    return (0.2126*r + 0.7152*g + 0.0722*b) / 255 * 100

def patch(cx, cy, rad=6):
    """avg RGB over a (2rad+1)^2 box, clamped to frame"""
    rs = gs = bs = n = 0
    for y in range(max(0, cy-rad), min(H, cy+rad+1)):
        for x in range(max(0, cx-rad), min(W, cx+rad+1)):
            r, g, b = px[x, y]
            rs += r; gs += g; bs += b; n += 1
    r, g, b = rs/n, gs/n, bs/n
    return r, g, b

print(f"# ref = {W}x{H}\n")

# --- named sample points (fractions of W,H so it's resolution-independent) ---
pts = {
    "G6 sunlit floor patch (beam on table, fg-left)": (0.30, 0.74),
    "G6 sunlit counter patch (mid-left ledge)":        (0.14, 0.66),
    "Pass1 window-light bar on floor":                 (0.34, 0.80),
    "G3 bounce shadow (open shade, wall mid)":          (0.55, 0.45),
    "hero bowl lit side":                               (0.30, 0.86),
    "teal block body":                                  (0.78, 0.83),
    "fridge stainless mid":                             (0.88, 0.40),
    "wood counter warm mid":                            (0.60, 0.90),
    "back cabinets shade":                              (0.62, 0.30),
}
print("## named patches (avg of 13x13 box)")
for name, (fx, fy) in pts.items():
    x, y = int(fx*W), int(fy*H)
    r, g, b = patch(x, y)
    print(f"  {name:48s} @({x:4d},{y:4d})  RGB=({r:5.1f},{g:5.1f},{b:5.1f})  L={L(r,g,b):5.1f}%  R-B={r-b:+6.1f}")

# --- G3: darkest point in frame (deep shade) — scan for min-luminance patch ---
print("\n## G3 darkest-patch scan (min avg-L over 9x9 boxes, step 12)")
best = None
for y in range(6, H-6, 12):
    for x in range(6, W-6, 12):
        r, g, b = patch(x, y, 4)
        lum = L(r, g, b)
        if best is None or lum < best[0]:
            best = (lum, x, y, r, g, b)
lum, x, y, r, g, b = best
print(f"  darkest @({x},{y})  RGB=({r:.1f},{g:.1f},{b:.1f})  L={lum:.2f}%  warm(R>=G>=B)={r>=g>=b}  R-B={r-b:+.1f}")

# --- G5 / Pass6: brightest window pixels + 3-point gradient ---
print("\n## G5 window highlight (brightest single px + 3-pt gradient down the pane)")
bmax = (-1, 0, 0)
# window pane region roughly left 4-18% wide, 12-55% tall
for y in range(int(0.12*H), int(0.55*H), 3):
    for x in range(int(0.03*W), int(0.20*W), 3):
        r, g, b = px[x, y]
        m = max(r, g, b)
        if m > bmax[0]:
            bmax = (m, x, y)
m, bx, by = bmax
print(f"  brightest px @({bx},{by})  max-channel={m}  RGB={px[bx,by]}")
for i, fy in enumerate([0.18, 0.34, 0.50]):
    x, y = int(0.09*W), int(fy*H)
    r, g, b = px[x, y]
    print(f"    grad pt{i+1} @({x},{y})  RGB=({r},{g},{b})  L={L(r,g,b):.1f}%")

# --- Pass9: whole-frame warm/cool + teal area ---
print("\n## Pass9 palette / warm-cool balance (full frame, step 4)")
Rs = Gs = Bs = n = 0
warm = teal = 0
for y in range(0, H, 4):
    for x in range(0, W, 4):
        r, g, b = px[x, y]
        Rs += r; Gs += g; Bs += b; n += 1
        if r >= b: warm += 1
        # teal-ish: green dominant & bluish & not too dark
        if g > r+8 and g > 40 and b > r-10 and (g+b) > 2*r:
            teal += 1
print(f"  channel means: R={Rs/n:.1f}  G={Gs/n:.1f}  B={Bs/n:.1f}   (R-B={Rs/n-Bs/n:+.1f})")
print(f"  warm pixels (R>=B): {warm/n*100:.1f}%   teal-ish pixels: {teal/n*100:.2f}%")
