import os
import sys
import numpy as np
from PIL import Image, ImageFilter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

# HARD GUARD — whole-frame luminance/warmth/DOF stats compared against the
# golden ref, so a HUD in CUR is a difference the ref does not have and every
# delta printed below is that HUD. The frame used to be HARDCODED
# (`CUR = "hero-look-final.png"`), which is the same defect one step worse than
# an implicit argv default: you could not even tell from the command line which
# frame produced the numbers. Now it must be named, and named de-HUDded.
#   Usage: grade_beauty.py <frame>-nohud2.png
# REF is exempt on purpose: the golden ref is curated artwork, never a capture.
require_nohud2(sys.argv[1:2], tool="grade_beauty.py")

REF = "docs/assets/golden-beauty-shot-ref.png"
CUR = sys.argv[1]

def load(p, size=(1024,1024)):
    im = Image.open(p).convert("RGB").resize(size, Image.LANCZOS)
    return im

def stats(name, im):
    a = np.asarray(im).astype(np.float32)
    R,G,B = a[...,0],a[...,1],a[...,2]
    # luminance (Rec.709)
    L = 0.2126*R+0.7152*G+0.0722*B
    print(f"\n===== {name} =====")
    print(f"Lum: mean {L.mean():6.1f}  median {np.median(L):6.1f}  p05 {np.percentile(L,5):6.1f} ({np.percentile(L,5)/255*100:4.1f}%)  p95 {np.percentile(L,95):6.1f}  std {L.std():5.1f}")
    print(f"Lum range p95-p05 (dynamic range): {np.percentile(L,95)-np.percentile(L,5):5.1f}")
    # warmth global
    print(f"Warmth R-B: mean {(R-B).mean():6.1f}   R {R.mean():5.1f} G {G.mean():5.1f} B {B.mean():5.1f}")
    # saturation (HSV S approx)
    mx = a.max(axis=2); mn = a.min(axis=2)
    sat = np.where(mx>0,(mx-mn)/np.maximum(mx,1e-6),0)
    print(f"Saturation: mean {sat.mean()*100:4.1f}%  p50 {np.median(sat)*100:4.1f}%  p90 {np.percentile(sat,90)*100:4.1f}%")
    # shadow warmth: darkest 15% pixels
    thr = np.percentile(L,15)
    m = L<=thr
    print(f"Shadow (darkest15%): L {L[m].mean():5.1f} ({L[m].mean()/255*100:4.1f}%)  R-B {(R[m]-B[m]).mean():6.1f}  R {R[m].mean():5.1f} G {G[m].mean():5.1f} B {B[m].mean():5.1f}")
    # highlight (brightest 5%) clip check
    hi = np.percentile(L,99)
    clip = (L>=254).mean()*100
    print(f"Highlight: p99 {hi:5.1f}  %pixels>=254 (clip) {clip:4.2f}%")
    # micro-contrast: local std via high-pass (laplacian-ish)
    g = np.asarray(im.convert("L")).astype(np.float32)
    blur = np.asarray(im.convert("L").filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    hp = g-blur
    print(f"Micro-contrast (high-pass std, r3): {hp.std():5.2f}")
    # global edge energy (sharpness proxy) via gradient magnitude
    gx = np.abs(np.diff(g,axis=1)); gy=np.abs(np.diff(g,axis=0))
    print(f"Edge energy (mean|grad|): {gx.mean()*0.5+gy.mean()*0.5:5.2f}")
    return a, L

def region_sharpness(name, im):
    # compare foreground(bottom-center) vs background(top region) high-freq to gauge DOF
    g = np.asarray(im.convert("L")).astype(np.float32)
    H,W = g.shape
    def hf(sub):
        b = np.asarray(Image.fromarray(sub.astype(np.uint8)).filter(ImageFilter.GaussianBlur(2))).astype(np.float32)
        return (sub-b).std()
    fg = g[int(H*0.72):int(H*0.95), int(W*0.30):int(W*0.70)]   # hero object zone
    bg = g[int(H*0.10):int(H*0.40), int(W*0.35):int(W*0.85)]   # far cabinets/fridge
    fgv, bgv = hf(fg), hf(bg)
    print(f"[{name}] DOF sep  fg-hf {fgv:5.2f}  bg-hf {bgv:5.2f}  ratio fg/bg {fgv/max(bgv,1e-3):4.2f}")

def vignette(name, im):
    a = np.asarray(im.convert("L")).astype(np.float32)
    H,W = a.shape
    cy,cx = H//2, W//2
    center = a[cy-80:cy+80, cx-80:cx+80].mean()
    corners = np.mean([a[:120,:120].mean(), a[:120,-120:].mean(), a[-120:,:120].mean(), a[-120:,-120:].mean()])
    print(f"[{name}] Vignette center {center:5.1f} corners {corners:5.1f}  drop {(1-corners/max(center,1))*100:5.1f}%")

ref = load(REF); cur = load(CUR)
stats("GOLDEN REF", ref)
stats("CURRENT (hero-look-final)", cur)
print("\n----- DOF / depth separation -----")
region_sharpness("REF", ref); region_sharpness("CUR", cur)
print("\n----- Vignette -----")
vignette("REF", ref); vignette("CUR", cur)
