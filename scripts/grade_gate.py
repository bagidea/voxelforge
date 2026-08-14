#!/usr/bin/env python3
"""Full numeric GATE readout for ANY frame (G3 + G5 + G6 — the three the rubric grades
by eyedropper). Auto-locates the window highlight so it works on any framing, not just
the ref. Prints PASS/FAIL per gate so a G3 fix can be confirmed to not regress G5/G6.
Usage: grade_gate.py <frame>-nohud2.png"""
import os
import sys
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

# HARD GUARD — refuse anything that is not a de-HUDded frame, before a single
# number is printed. The `[E]` prompt glyphs are ~250 and sit on the ground, so a
# raw --play capture false-FAILs G5 and drags p95 up to the UI. See nohud2_guard.
# (The old `argv[1] or "tune-v2.png"` default is gone with it: an implicit frame
# is exactly how a stale capture gets graded without anyone naming it.)
require_nohud2(sys.argv[1:2], tool="grade_gate.py")

path = sys.argv[1]
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

m = int(0.08 * W)
print(f"# {path} = {W}x{H}\n")

# ---------- G3: interior shade floor + tone ----------
vals = []
for y in range(m, H-m, 3):
    for x in range(m, W-m, 3):
        vals.append(Lum(*px[x, y]))
vals.sort()
p05 = vals[int(len(vals)*0.05)]
p50 = vals[int(len(vals)*0.50)]
best = None
for y in range(m, H-m, 10):
    for x in range(m, W-m, 10):
        r, g, b = patch(x, y, 5)
        lum = Lum(r, g, b)
        if best is None or lum < best[0]:
            best = (lum, x, y, r, g, b)
_, dx, dy, dr, dg, db = best
g3 = (p05 >= 8) and (dr >= db)
print("## G3  bounce/shade not black, not blue")
print(f"   interior p05-L={p05:.1f}% (need >=8)  p50-L={p50:.1f}%")
print(f"   darkest shade @({dx},{dy}) RGB=({dr:.0f},{dg:.0f},{db:.0f}) R-B={dr-db:+.0f} warm={dr>=db}")
print(f"   -> {'PASS' if g3 else 'FAIL'}\n")

# ---------- G5: window highlight not a flat 255 plate ----------
# Locate the window WITHOUT looking at the gradient, so G5's gradient test can still
# FAIL. The window/glare is the brightest NEAR-NEUTRAL region (glass is R~=G~=B); a
# saturated golden ceiling/wall plate (R>G>B, large R-B) is bright too but is NOT the
# window. Selecting on chroma keeps `spread>=8` a genuine, independent test.
# (The prior fix pre-selected the locate pixel for vspread>=8 and then re-checked
#  spread>=8 there with the same 3-pt formula -> tautology; the gradient sub-check
#  could never fail. Chroma-locate removes that circularity while still steering
#  clear of the flat golden corner that made plain-brightest false-FAIL on warm
#  framings.)
NEUTRAL_RB = 45     # |R-B| <= this == glassy/neutral, not golden plate
bmax = (-1, 0, 0)   # fallback: plain brightest, any hue
bwin = (-1, 0, 0)   # brightest GLASSY pixel: bright (>=200) + near-neutral hue
for y in range(0, H, 3):
    for x in range(0, W, 3):
        r, g, b = px[x, y]
        mx = max(r, g, b)
        if mx > bmax[0]:
            bmax = (mx, x, y)
        if mx > bwin[0] and mx >= 200 and abs(r - b) <= NEUTRAL_RB:
            bwin = (mx, x, y)
_, wx, wy = bwin if bwin[0] > 0 else bmax
# 3-pt vertical gradient centred on the window
gpts = []
for dyy in (-int(0.10*H), 0, int(0.10*H)):
    yy = min(H-1, max(0, wy+dyy))
    gpts.append(px[wx, yy])
Ls = [Lum(*p) for p in gpts]
spread = max(Ls) - min(Ls)
br = px[wx, wy]
# brightest px: is at least one of G/B rolled off (<=245)? and gradient spread >=8?
gb_rolloff = min(br[1], br[2]) <= 245
g5 = gb_rolloff and spread >= 8
print("## G5  window not blown-out (gradient survives, G/B roll off)")
print(f"   brightest px @({wx},{wy}) RGB={br}  min(G,B)={min(br[1],br[2])} (need <=245)")
print(f"   3-pt vertical L = {[round(l,1) for l in Ls]}  spread={spread:.1f} (need >=8)")
print(f"   -> {'PASS' if g5 else 'FAIL'}\n")

# ---------- G6: warm golden sunlit wood patch ----------
# Does a golden sunlit patch EXIST? (rubric intent) — not "is the single brightest
# warm patch golden". A brighter near-white wall (R-B<40) used to mask a real golden
# wall that was present, false-FAILing G6. So: gather GOLDEN patches (R>G>B, R-B 40..210)
# in the lower 55%, take the brightest one, and require it be sunlit-bright (L>=55).
# No window-column half-exclusion: the near-white window glare is R~=G~=B so it
# never satisfies R-B>=40 anyway.
cands = []
for y in range(int(0.45*H), H-6, 8):
    for x in range(6, W-6, 8):
        r, g, b = patch(x, y, 4)
        if r > g > b and 40 <= (r - b) <= 210:
            cands.append((Lum(r, g, b), x, y, r, g, b))
cands.sort(reverse=True)
if cands:
    sl, sx, sy, sr, sg, sb = cands[0]
    rb = sr - sb
    g6 = sl >= 55
    print("## G6  warm golden tone on sunlit wood (R>G>B, R-B 40..210, sunlit L>=55)")
    print(f"   brightest golden patch @({sx},{sy}) RGB=({sr:.0f},{sg:.0f},{sb:.0f}) R-B={rb:+.0f} L={sl:.1f}")
    print(f"   -> {'PASS' if g6 else 'FAIL'}\n")
else:
    g6 = False
    print("## G6  NO warm patch found -> FAIL\n")

print("=" * 44)
allp = g3 and g5 and g6
print(f"MEASURABLE GATES: G3={'P' if g3 else 'F'}  G5={'P' if g5 else 'F'}  G6={'P' if g6 else 'F'}"
      f"   => {'all measurable gates PASS' if allp else 'REGRESSION'}")
print("(G1 voxel-edge / G2 window-bars-on-floor / G4 soft-shadow+AO = visual check)")
sys.exit(0 if allp else 1)
