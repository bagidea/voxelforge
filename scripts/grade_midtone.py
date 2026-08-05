import os, sys, numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

# Midtone-band probe: the 3 P0 axes Pixel graded (kill blue-wash).
# Midtone = pixels in a central luminance band (exclude crushed shadows &
# blown highlights), measured on the same 1024 resample as grade_beauty.
#
# HARD GUARD — the band is picked by luminance percentile over the WHOLE frame,
# so HUD glyphs at ~250 shift the p35/p75 cut points themselves: every number
# below moves, not just the ones in the glyph pixels. Usage:
#   grade_midtone.py <frame>-nohud2.png [LO_PCT HI_PCT]
require_nohud2(sys.argv[1:2], tool="grade_midtone.py")

CUR = sys.argv[1]
LO = float(sys.argv[2]) if len(sys.argv) > 2 else 35.0
HI = float(sys.argv[3]) if len(sys.argv) > 3 else 75.0

def load(p, size=(1024,1024)):
    return Image.open(p).convert("RGB").resize(size, Image.LANCZOS)

def mid(name, im, lo, hi):
    a = np.asarray(im).astype(np.float32)
    R,G,B = a[...,0],a[...,1],a[...,2]
    L = 0.2126*R+0.7152*G+0.0722*B
    tlo, thi = np.percentile(L,lo), np.percentile(L,hi)
    m = (L>=tlo)&(L<=thi)
    mx=a.max(2); mn=a.min(2)
    sat=np.where(mx>0,(mx-mn)/np.maximum(mx,1e-6),0)
    print(f"[{name}] band L[{lo:.0f}-{hi:.0f}pct]=({tlo:.0f}..{thi:.0f})  "
          f"R {R[m].mean():5.1f}  G {G[m].mean():5.1f}  B {B[m].mean():5.1f}  "
          f"R-B {(R[m]-B[m]).mean():+6.1f}  sat {sat[m].mean()*100:4.1f}%")

for p in ([CUR] if len(sys.argv)>1 else ["docs/assets/golden-beauty-shot-ref.png","hero-look-final.png"]):
    mid(p, load(p), LO, HI)
