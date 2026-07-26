#!/usr/bin/env python3
"""Grade a hero frame against the 6 gates + monochrome-collapse check.
Usage: python scripts/grade_hero.py hero-XX.png
Style mirrors grade_ref2.py (PIL eyedrop, fixed sample logic)."""
import sys
from PIL import Image

path = sys.argv[1] if len(sys.argv) > 1 else "hero-01.png"
img = Image.open(path).convert("RGB")
W, H = img.size
px = img.load()

def Lum(r, g, b): return (0.2126*r + 0.7152*g + 0.0722*b)/255*100
def patch(cx, cy, rad=5):
    rs = gs = bs = n = 0
    for y in range(max(0, cy-rad), min(H, cy+rad+1)):
        for x in range(max(0, cx-rad), min(W, cx+rad+1)):
            r, g, b = px[x, y]; rs += r; gs += g; bs += b; n += 1
    return rs/n, gs/n, bs/n

print(f"== {path}  ({W}x{H}) ==")

# ---- G6 / Pass1b: brightest sunlit WOOD patch (warm, exclude window region) ----
# window is screen-left; exclude left 25% & only look at lower 55%..100% for the table/floor
cands = []
for y in range(int(0.45*H), H-6, 6):
    for x in range(int(0.28*W), W-6, 6):
        r, g, b = patch(x, y, 4)
        if r > g > b and Lum(r, g, b) < 92:  # warm, not a blown highlight
            cands.append((Lum(r, g, b), x, y, r, g, b))
cands.sort(reverse=True)
print("\n[G6] brightest sunlit wood patches (R>G>B):")
g6_ok = False
for lum, x, y, r, g, b in cands[:4]:
    rb = r-b
    ok = (r > g > b) and (40 <= rb <= 210)
    g6_ok = g6_ok or ok
    print(f"  @({x:4d},{y:4d}) RGB=({r:5.1f},{g:5.1f},{b:5.1f}) L={lum:4.1f}% R-B={rb:+6.1f} {'OK' if ok else 'x'}")
print(f"  G6 warm-golden (R>G>B & R-B 40..210): {'PASS' if g6_ok else 'FAIL'}")

# ---- G5/G6a: window highlight — 3 vertical samples down the bright pane ----
# find brightest column band on screen-left third
bx = by = bl = -1
for y in range(int(0.02*H), int(0.75*H), 5):
    for x in range(int(0.01*W), int(0.30*W), 5):
        r, g, b = patch(x, y, 3)
        l = Lum(r, g, b)
        if l > bl:
            bl, bx, by = l, x, y
print(f"\n[G5] window brightest @({bx},{by}) L={bl:.1f}%")
samples = []
for dy in (-int(0.12*H), 0, int(0.12*H)):
    yy = min(H-4, max(4, by+dy))
    r, g, b = patch(bx, yy, 3)
    samples.append((yy, r, g, b, Lum(r, g, b)))
    print(f"  @({bx},{yy}) RGB=({r:5.1f},{g:5.1f},{b:5.1f}) L={Lum(r,g,b):5.1f}%")
br, bg, bb = samples[[s[4] for s in samples].index(max(s[4] for s in samples))][1:4]
gb_min = min(bg, bb)
Ls = [s[4] for s in samples]
grad = max(Ls) - min(Ls)
g5_ok = (gb_min <= 245) and (grad >= 8)
print(f"  brightest G/B min={gb_min:.0f} (<=245?) gradient={grad:.1f}L (>=8?): {'PASS' if g5_ok else 'FAIL'}")

# ---- G3: interior luminance floor + warm/not-blue ----
m = int(0.08*W)
mh = int(0.08*H)
vals = []
for y in range(mh, H-mh, 3):
    for x in range(m, W-m, 3):
        r, g, b = px[x, y]; vals.append(Lum(r, g, b))
vals.sort()
p05 = vals[int(len(vals)*0.05)]
p10 = vals[int(len(vals)*0.10)]
# darkest representative patch tone
best = None
for y in range(mh, H-mh, 8):
    for x in range(m, W-m, 8):
        r, g, b = patch(x, y, 5)
        l = Lum(r, g, b)
        if best is None or l < best[0]: best = (l, x, y, r, g, b)
dl, dx, dy, dr, dg, db = best
warm = dr >= dg >= db
notblue = db <= dr
g3_ok = (p05 >= 8) and warm and notblue
print(f"\n[G3] interior p05-L={p05:.1f}% p10-L={p10:.1f}% (job floor >=10)")
print(f"  darkest patch @({dx},{dy}) RGB=({dr:.1f},{dg:.1f},{db:.1f}) warm(R>=G>=B)={warm} R-B={dr-db:+.1f}")
print(f"  G3 (p05>=8 & warm & not-blue): {'PASS' if g3_ok else 'FAIL'}  |  floor>=10%: {'yes' if p05>=10 else 'no'}")

# ---- monochrome-collapse: material identity survives ----
print("\n[MONO] material identity:")
# green-dominant share (accent block visible)
gn = 0; tot = 0; gx = gy = 0; gmax = None
for y in range(0, H, 3):
    for x in range(0, W, 3):
        r, g, b = px[x, y]; tot += 1
        if g >= r and g >= b and g > 25:
            gn += 1; gx += x; gy += y
            if gmax is None or g > gmax[2]: gmax = (x, y, g, r, b)
share = gn/tot*100
print(f"  green-dominant share = {share:.2f}%  (accent visible if >0.2%)")
if gmax:
    x, y, g, r, b = gmax
    rr, gg, bb = patch(x, y, 6)
    print(f"    greenest @({x},{y}) patch RGB=({rr:.1f},{gg:.1f},{bb:.1f})")
# whole-frame channel means
rs = gs = bs = 0; n = 0
for y in range(0, H, 4):
    for x in range(0, W, 4):
        r, g, b = px[x, y]; rs += r; gs += g; bs += b; n += 1
print(f"  frame mean RGB=({rs/n:.0f},{gs/n:.0f},{bs/n:.0f})  warm-lean R-B={(rs-bs)/n:+.0f}")

print("\n== SUMMARY ==")
print(f"  G3 bounce   : {'PASS' if g3_ok else 'FAIL'}")
print(f"  G5 no-clip  : {'PASS' if g5_ok else 'FAIL'}")
print(f"  G6 warm     : {'PASS' if g6_ok else 'FAIL'}")
print(f"  green share : {share:.2f}%")
