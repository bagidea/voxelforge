#!/usr/bin/env python3
"""Generic G3 grader for ANY frame. Measures the two things the rubric's G3 gate
needs from one image: interior p05 luminance (is the whole shade crushed to black?)
and the darkest representative shade patch's tone (warm R>=G>=B, or gone cold/blue?).
Usage: grade_g3.py <frame.png>"""
import sys
from PIL import Image

path = sys.argv[1] if len(sys.argv) > 1 else "tune-v2.png"
img = Image.open(path).convert("RGB")
W, H = img.size
px = img.load()

def Lum(r, g, b):
    return (0.2126*r + 0.7152*g + 0.0722*b) / 255 * 100

def patch(cx, cy, rad=5):
    rs = gs = bs = n = 0
    for y in range(max(0, cy-rad), min(H, cy+rad+1)):
        for x in range(max(0, cx-rad), min(W, cx+rad+1)):
            r, g, b = px[x, y]
            rs += r; gs += g; bs += b; n += 1
    return rs/n, gs/n, bs/n

m = int(0.08 * W)   # trim outer 8% (rubric: avoid frame-corner crevices)
print(f"# {path} = {W}x{H}  (interior = trim outer 8%)\n")

# interior luminance percentiles
vals = []
for y in range(m, H-m, 3):
    for x in range(m, W-m, 3):
        r, g, b = px[x, y]
        vals.append(Lum(r, g, b))
vals.sort()
print("## interior luminance percentiles")
pcs = {}
for p in [1, 5, 10, 50, 90, 99]:
    v = vals[int(len(vals)*p/100)]
    pcs[p] = v
    print(f"  p{p:02d} L = {v:5.1f}%")

# representative deep shade: darkest 11x11 interior patch + its tone
best = None
for y in range(m, H-m, 10):
    for x in range(m, W-m, 10):
        r, g, b = patch(x, y, 5)
        lum = Lum(r, g, b)
        if best is None or lum < best[0]:
            best = (lum, x, y, r, g, b)
lum, x, y, r, g, b = best
warm = r >= g >= b
print(f"\n## darkest representative shade @({x},{y})")
print(f"  RGB=({r:.1f},{g:.1f},{b:.1f})  L={lum:.2f}%  warm(R>=G>=B)={warm}  R-B={r-b:+.1f}")

# whole-frame warm/cool means (context)
Rs = Gs = Bs = n = 0
for y in range(0, H, 4):
    for x in range(0, W, 4):
        r, g, b = px[x, y]
        Rs += r; Gs += g; Bs += b; n += 1
print(f"\n## frame means  R={Rs/n:.1f} G={Gs/n:.1f} B={Bs/n:.1f}  (R-B={Rs/n-Bs/n:+.1f})")

# verdict
p05 = pcs[5]
cold = b > r
print("\n## G3 GATE")
print(f"  p05-L >= 8% ? {p05:.1f}%  -> {'PASS' if p05 >= 8 else 'FAIL'}")
print(f"  darkest shade warm (not blue, R>=B) ? R-B={r-b:+.1f} -> {'PASS' if r >= b else 'FAIL (cold/blue)'}")
g3 = (p05 >= 8) and (r >= b)
print(f"  == G3: {'PASS' if g3 else 'FAIL'} ==")
