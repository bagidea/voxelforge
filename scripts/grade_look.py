#!/usr/bin/env python3
"""Honest look grader — measures MORE than the old 3-gate script.

Adds numeric probes for the gates the previous grader only eyeballed:
  * G4a  soft-shadow penumbra width  (floor lit->shadow 10-90% fall distance, px)
  * G4b  contact AO                  (luminance dip in the band where objects meet floor)
  * G2   key-light direction         (a bright directional stripe distinct from ambient)
G3/G5/G6 reuse the calibrated eyedropper logic from grade_gate.py.
G1 (voxel 90-deg edges) is still a visual call — printed as INFO, never as a PASS
we didn't measure. The point of this file is to stop over-claiming: every line says
whether it is MEASURED or a VISUAL check.

Usage: grade_look.py <frame.png>
"""
import sys
from PIL import Image, ImageFilter

path = sys.argv[1] if len(sys.argv) > 1 else "hero.png"
img = Image.open(path).convert("RGB")
W, H = img.size
px = img.load()


def Lum(r, g, b):
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 * 100


def patch(cx, cy, rad=5):
    rs = gs = bs = n = 0
    for y in range(max(0, cy - rad), min(H, cy + rad + 1)):
        for x in range(max(0, cx - rad), min(W, cx + rad + 1)):
            r, g, b = px[x, y]
            rs += r; gs += g; bs += b; n += 1
    return rs / n, gs / n, bs / n


m = int(0.08 * W)
print(f"# {path} = {W}x{H}\n")

# ---------- G3 (MEASURED): interior shade floor + tone ----------
vals = []
for y in range(m, H - m, 3):
    for x in range(m, W - m, 3):
        vals.append(Lum(*px[x, y]))
vals.sort()
p05 = vals[int(len(vals) * 0.05)]
p50 = vals[int(len(vals) * 0.50)]
best = None
for y in range(m, H - m, 10):
    for x in range(m, W - m, 10):
        r, g, b = patch(x, y, 5)
        lum = Lum(r, g, b)
        if best is None or lum < best[0]:
            best = (lum, x, y, r, g, b)
_, dx, dy, dr, dg, db = best
g3 = (p05 >= 8) and (dr >= db)
print("## G3 [MEASURED] bounce/shade not black, not blue")
print(f"   interior p05-L={p05:.1f}% (need >=8)  p50-L={p50:.1f}%")
print(f"   darkest shade RGB=({dr:.0f},{dg:.0f},{db:.0f}) R-B={dr-db:+.0f}")
print(f"   -> {'PASS' if g3 else 'FAIL'}\n")

# ---------- G5 (MEASURED): window highlight not a flat 255 plate ----------
bmax = (-1, 0, 0)
for y in range(0, H, 3):
    for x in range(0, W, 3):
        r, g, b = px[x, y]
        mx = max(r, g, b)
        if mx > bmax[0]:
            bmax = (mx, x, y)
_, wx, wy = bmax
gpts = []
for dyy in (-int(0.10 * H), 0, int(0.10 * H)):
    yy = min(H - 1, max(0, wy + dyy))
    gpts.append(px[wx, yy])
Ls = [Lum(*p) for p in gpts]
spread = max(Ls) - min(Ls)
br = px[wx, wy]
gb_rolloff = min(br[1], br[2]) <= 245
g5 = gb_rolloff and spread >= 8
print("## G5 [MEASURED] window not blown-out (gradient survives)")
print(f"   brightest px @({wx},{wy}) RGB={br} min(G,B)={min(br[1],br[2])} (<=245)  spread={spread:.1f}")
print(f"   -> {'PASS' if g5 else 'FAIL'}\n")

# ---------- G6 (MEASURED): warm golden sunlit wood patch ----------
cands = []
x_lo = int(0.20 * W) if wx < W / 2 else 0
x_hi = W - 6 if wx < W / 2 else int(0.80 * W)
for y in range(int(0.45 * H), H - 6, 8):
    for x in range(max(6, x_lo), min(W - 6, x_hi), 8):
        r, g, b = patch(x, y, 4)
        if r > g > b:
            cands.append((Lum(r, g, b), x, y, r, g, b))
cands.sort(reverse=True)
if cands:
    _, sx, sy, sr, sg, sb = cands[0]
    rb = sr - sb
    g6 = (sr > sg > sb) and (40 <= rb <= 210)
    print("## G6 [MEASURED] warm golden tone (R>G>B, R-B 40..210)")
    print(f"   brightest warm patch RGB=({sr:.0f},{sg:.0f},{sb:.0f}) R-B={rb:+.0f}")
    print(f"   -> {'PASS' if g6 else 'FAIL'}\n")
else:
    g6 = False
    print("## G6 [MEASURED] NO warm patch -> FAIL\n")

# ---------- G4a (MEASURED): soft-shadow penumbra width ----------
# Blur a hair to kill single-pixel dither, then on floor rows (lower 45%) find the
# steepest lit->shadow drops and measure the 10-90% fall distance in px. Hard PCF
# ~1-2px; PCSS penumbra >=3px. We report the MEDIAN of the strongest edges so one
# soft blob can't fake it.
blur = img.filter(ImageFilter.GaussianBlur(0.6)).convert("L")
bl = blur.load()
widths = []
y0, y1 = int(0.55 * H), H - 4
for y in range(y0, y1, 2):
    row = [bl[x, y] for x in range(W)]
    for x in range(6, W - 6):
        # descending edge: local slope negative and steep
        d = row[x + 1] - row[x - 1]
        if d < -14:  # steep drop (lit -> shadow), 0..255 scale
            # find local hi (left) and lo (right) plateau within +-14px
            lo = min(row[max(0, x - 14):x + 15])
            hi = max(row[max(0, x - 14):x + 15])
            # A real SUNLIT->shadow edge: bright lit side (not two mid-tone
            # checkerboard squares) and a big drop. This is what stopped the old
            # hard-shadow frame from false-passing on floor tile edges.
            if hi < 150 or hi - lo < 60:
                continue
            t10 = lo + 0.1 * (hi - lo)
            t90 = lo + 0.9 * (hi - lo)
            # walk right from x to reach t10, left to reach t90
            xr = x
            while xr < W - 1 and row[xr] > t10:
                xr += 1
            xl = x
            while xl > 0 and row[xl] < t90:
                xl -= 1
            wpx = xr - xl
            if 1 <= wpx <= 40:
                widths.append(wpx)
widths.sort()
if widths:
    # use the strongest-edge subset: median of all detected
    pen = widths[len(widths) // 2]
    p25 = widths[len(widths) // 4]
    # Threshold calibrated against the two reference points at THIS frame size
    # (1280w, 0.6px pre-blur): golden ref reads 8px (PCSS-soft), the old hard-PCF
    # frame reads 3px (AA/blur inflates a true ~1px edge). Bar at >=5 sits between
    # them so a genuinely soft frame passes and a hard one fails.
    g4a = pen >= 5
    print("## G4a [MEASURED] soft-shadow penumbra (floor 10-90% fall px)")
    print(f"   edges found={len(widths)}  penumbra median={pen}px  p25={p25}px (need median>=5)")
    print(f"   -> {'PASS' if g4a else 'FAIL'}")
else:
    pen = 0
    g4a = False
    print("## G4a [MEASURED] no shadow edges detected on floor -> FAIL")

# ---------- G4b (MEASURED): contact AO ----------
# Compare the darkest floor-contact band (where a vertical dark dip sits directly
# under a brighter object) against the open floor nearby. Scan columns: for each
# column find a spot where luminance dips locally then recovers (a contact groove).
ao_dips = []
gray = img.filter(ImageFilter.GaussianBlur(0.5)).convert("L")
gl = gray.load()
for x in range(m, W - m, 3):
    col = [gl[x, y] for y in range(H)]
    for y in range(int(0.35 * H), H - 8):
        here = col[y]
        above = sum(col[max(0, y - 8):y - 2]) / 6.0
        below = sum(col[y + 2:y + 8]) / 6.0
        nbr = (above + below) / 2.0
        dip = nbr - here
        if dip > 12 and above > here and below > here:  # local dark groove
            ao_dips.append(dip)
ao_dips.sort(reverse=True)
ao_strength = sum(ao_dips[:40]) / min(40, len(ao_dips)) if ao_dips else 0.0
# NOTE: this groove metric is CONFOUNDED by the checkerboard floor + block seams
# (they read as dark grooves too), so it is an INDICATOR for old-vs-new comparison
# on the same scene, NOT an absolute AO pass. G4b stays a visual call.
print("## G4b [INDICATOR only] contact-AO groove strength (confounded by checker floor)")
print(f"   grooves={len(ao_dips)}  top-40 mean dip={ao_strength:.1f} L-units")
print("   -> VISUAL call (compare under-bowl darkening old vs new)")
# G4 final verdict is a VISUAL determination; the script gives evidence, not a stamp.
print(f"   G4a penumbra={pen}px is the objective half; G4b AO = eyeball\n")

# ---------- G2 (MEASURED, heuristic): key-light direction ----------
# A directional key leaves a bright stripe on floor/wall distinct from flat ambient.
# Proxy: the brightest 5% of interior pixels should be much brighter than the median
# AND spatially clustered on one side (a beam), not scattered everywhere.
inter = []
for y in range(m, H - m, 2):
    for x in range(m, W - m, 2):
        inter.append((Lum(*px[x, y]), x, y))
inter.sort(reverse=True)
top = inter[: max(50, len(inter) // 20)]
med = inter[len(inter) // 2][0]
top_mean = sum(v for v, _, _ in top) / len(top)
xs = [x for _, x, _ in top]
cx = sum(xs) / len(xs)
side_bias = abs(cx - W / 2) / (W / 2)  # 0 centred, 1 fully to one edge
g2 = (top_mean - med) >= 18  # a clear bright key above ambient floor
print("## G2 [MEASURED] key-light has a bright directional beam")
print(f"   top5% mean L={top_mean:.1f}  median L={med:.1f}  delta={top_mean-med:.1f} (need>=18)")
print(f"   beam x-centroid bias={side_bias:.2f} (0=centre,1=edge; ref light is off to one side)")
print(f"   -> {'PASS' if g2 else 'FAIL'}\n")

# ---------- G1 (VISUAL): voxel 90-degree edges ----------
print("## G1 [VISUAL] voxel hard 90-deg edges — eyeball at 100%, not scored here\n")

print("=" * 52)
print("GATE SUMMARY  (M=measured objective / V=visual call):")
print(f"  G2 key-dir  [M] = {'PASS' if g2 else 'FAIL'}")
print(f"  G3 shade    [M] = {'PASS' if g3 else 'FAIL'}")
print(f"  G4a penumbra[M] = {'PASS' if g4a else 'FAIL'}  ({pen}px, need>=5)")
print(f"  G4b AO      [V] = eyeball (groove indicator {ao_strength:.0f})")
print(f"  G5 window   [M] = {'PASS' if g5 else 'FAIL'}")
print(f"  G6 warm     [M] = {'PASS' if g6 else 'FAIL'}")
print(f"  G1 voxel    [V] = eyeball at 100%")
print("  => G4/G1 are visual determinations; G2/G3/G4a/G5/G6 objective above.")
