"""Depth-layering audit for gate3 frames vs the golden ref (Flamingo, 2026-08-07).

Question: does the scene actually separate foreground / midground / background,
or is it one flat slab? Measured, not eyeballed.

  (1) per depth-band local RMS contrast + mean saturation + mean luminance
      (bands = near/mid/far by screen y, the usual proxy for a grounded camera)
      + luminance-histogram OVERLAP between near and far. High overlap = the
      bands live in the same value range = no atmospheric perspective = flat.
  (2) edge-of-frame vs center-of-frame luminance (vignette / value falloff)
  (3) hero-vs-adjacent-background luminance delta (silhouette read)

Usage: python scripts/depth_layering.py   (needs numpy, pillow, scipy)

Started life as `scripts/_tmp_depth_layering.py`, a scratch file meant to be
deleted once the findings were written up. It is NOT deletable any more:
`docs/art-order-2026-08-07.md` promotes three of its outputs (corner drop,
radial-ring falloff, largest bright-island share) to acceptance gates that
Kevin / Monanisa / Shiba have to re-measure, so the tool has to survive in the
repo alongside the order. The old name was silently swallowed by `.gitignore`'s
`_[!_]*.py` scratch-file rule — hence the rename, not a `git add -f`.
"""
import os
import sys
import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REF = "docs/assets/golden-beauty-shot-ref.png"
SHOTS = [
    ("boot", "docs/assets/gate3/gate3-after-boot-nohud2.png"),
    ("walk", "docs/assets/gate3/gate3-after-walk-nohud2.png"),
    ("combat", "docs/assets/gate3/gate3-after-combat-nohud2.png"),
]


def load(p):
    return Image.open(os.path.join(ROOT, p)).convert("RGB")


def channels(im):
    a = np.asarray(im).astype(np.float32)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126 * R + 0.7152 * G + 0.0722 * B
    mx, mn = a.max(axis=2), a.min(axis=2)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    return a, L, sat


def local_rms(im):
    """local RMS contrast = std of (gray - gaussian(gray, r3)), per-pixel magnitude."""
    g = np.asarray(im.convert("L")).astype(np.float32)
    b = np.asarray(im.convert("L").filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    return np.abs(g - b)


def sky_mask(a):
    """blue-dominant = sky; it is 'far' but it is not scene material, report both ways."""
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    return (B > R + 12) & (B > G + 6)


def bands(name, im):
    a, L, sat = channels(im)
    hp = local_rms(im)
    H, W = L.shape
    sky = sky_mask(a)
    print(f"\n===== {name}  ({W}x{H}) =====")
    print(f"  sky pixels: {sky.mean()*100:4.1f}%")
    print("  band            | localRMS | sat%  | lum   | (sky-excluded: RMS / sat% / lum)")
    rows = {}
    cuts = [("FAR  (top 1/3)", 0.00, 0.33), ("MID  (mid 1/3)", 0.33, 0.66), ("NEAR (bot 1/3)", 0.66, 1.00)]
    for label, y0, y1 in cuts:
        s = slice(int(H * y0), int(H * y1))
        m = ~sky[s]
        r = (hp[s].mean(), sat[s].mean() * 100, L[s].mean(),
             hp[s][m].mean(), sat[s][m].mean() * 100, L[s][m].mean())
        rows[label[:4].strip()] = r
        print(f"  {label} | {r[0]:8.2f} | {r[1]:5.1f} | {r[2]:5.1f} | {r[3]:6.2f} / {r[4]:5.1f} / {r[5]:5.1f}")

    f, n = rows["FAR"], rows["NEAR"]
    def pct(a_, b_):
        return (b_ - a_) / max(a_, 1e-6) * 100.0
    print(f"  --> NEAR vs FAR (sky-excluded): localRMS {pct(f[3], n[3]):+6.1f}%   "
          f"sat {n[4]-f[4]:+5.1f}pp   lum {n[5]-f[5]:+6.1f}")

    # luminance histogram overlap near vs far (sky excluded) — 1.0 = identical value range
    hn = np.histogram(L[int(H*0.66):][~sky[int(H*0.66):]], bins=32, range=(0, 255), density=False)[0]
    hf = np.histogram(L[:int(H*0.33)][~sky[:int(H*0.33)]], bins=32, range=(0, 255), density=False)[0]
    hn = hn / max(hn.sum(), 1); hf = hf / max(hf.sum(), 1)
    print(f"  --> NEAR/FAR luminance histogram overlap: {np.minimum(hn, hf).sum()*100:5.1f}%  "
          f"(high = same value range = flat)")


def vignette(name, im):
    _, L, _ = channels(im)
    H, W = L.shape
    ew, eh = int(W * 0.12), int(H * 0.12)
    ring = np.ones_like(L, dtype=bool)
    ring[eh:H - eh, ew:W - ew] = False
    cy, cx = H // 2, W // 2
    ch, cw = int(H * 0.18), int(W * 0.18)
    center = L[cy - ch:cy + ch, cx - cw:cx + cw].mean()
    corners = np.mean([L[:eh, :ew].mean(), L[:eh, -ew:].mean(), L[-eh:, :ew].mean(), L[-eh:, -ew:].mean()])
    # radial profile — normalised elliptical radius, 5 rings. A real vignette falls
    # monotonically; scene content does not.
    yy, xx = np.mgrid[0:H, 0:W]
    r = np.sqrt(((yy - cy) / (H / 2)) ** 2 + ((xx - cx) / (W / 2)) ** 2) / np.sqrt(2)
    prof = [L[(r >= i / 5) & (r < (i + 1) / 5)].mean() for i in range(5)]
    mono = all(prof[i] >= prof[i + 1] for i in range(4))
    print(f"  [{name}] center {center:5.1f}  edge-ring {L[ring].mean():5.1f}  corners {corners:5.1f}  "
          f"corner drop {(1 - corners / max(center, 1)) * 100:5.1f}%")
    print(f"           radial L r0->r1: " + " ".join(f"{p:5.1f}" for p in prof) +
          f"   monotonic falloff: {'YES' if mono else 'NO'}   "
          f"r0-r4 drop {(1 - prof[4] / max(prof[0], 1)) * 100:+5.1f}%")


def bright_islands(name, im, pct=90, min_frac=0.15):
    """Count SEPARATE bright regions (L > p90), the thing p95 cannot see.

    p95 answers "how bright is the brightest pixel"; it says nothing about
    whether that brightness is one continuous pool the eye can land on or five
    scattered patches that split the read. Blobs smaller than `min_frac` % of
    the frame are speckle, not competing focal points, so they are dropped.
    """
    _, L, _ = channels(im)
    H, W = L.shape
    thr = np.percentile(L, pct)
    m = L > thr
    # close 1-px gaps so a pool broken by a toon outline stays one island
    m = ndimage.binary_closing(m, np.ones((5, 5)))
    lab, n = ndimage.label(m)
    if n == 0:
        print(f"  [{name}] no bright pixels above p{pct}")
        return
    sizes = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
    floor = H * W * min_frac / 100.0
    keep = sizes[sizes >= floor]
    keep = np.sort(keep)[::-1]
    tot = keep.sum()
    share = [f"{s / max(tot, 1) * 100:.0f}%" for s in keep[:6]]
    print(f"  [{name}] p{pct} = L {thr:5.1f}   islands >= {min_frac}% frame: "
          f"{len(keep):2d}   (raw blobs {n})")
    print(f"           largest-island share of all bright px: "
          f"{(keep[0] / max(tot, 1) * 100 if len(keep) else 0):5.1f}%   "
          f"per-island split: {' '.join(share)}")


def ink_density(name, im):
    """dark toon-outline pixels per band. A depth cue needs ink to THIN with distance;
    if the density is flat across bands the outline is depth-uniform and reads as decal."""
    a, L, _ = channels(im)
    g = np.asarray(im.convert("L")).astype(np.float32)
    blur = np.asarray(im.convert("L").filter(ImageFilter.GaussianBlur(2))).astype(np.float32)
    ink = (g < blur - 12) & (L < 70)
    H = L.shape[0]
    d = [ink[int(H * a0):int(H * b0)].mean() * 100 for a0, b0 in ((0, .33), (.33, .66), (.66, 1.))]
    print(f"  [{name}] ink% far {d[0]:5.2f}  mid {d[1]:5.2f}  near {d[2]:5.2f}   "
          f"near/far ratio {d[2] / max(d[0], 1e-6):4.2f}")


def hero_delta(name, im, mode):
    """Isolate the hero object, compare its luminance to the pixels immediately touching it."""
    a, L, sat = channels(im)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    H, W = L.shape
    if mode == "capsule":     # red/orange placeholder avatar (sampled: R=255,G~90,B~28)
        m = (R > 195) & (B < 95) & (G < R * 0.82) & (sat > 0.68)
        m = ndimage.binary_closing(m, np.ones((5, 5)))
        keep = np.zeros_like(m); keep[int(H * .35):int(H * .80), int(W * .35):int(W * .70)] = True
    else:                     # golden ref: the cream bowl is the foreground hero prop
        m = np.zeros_like(L, dtype=bool)
        box = L[int(H * .74):int(H * .96), int(W * .17):int(W * .55)]
        m[int(H * .74):int(H * .96), int(W * .17):int(W * .55)] = box > np.percentile(box, 55)
        keep = np.ones_like(m)
    lab, n = ndimage.label(m & keep)
    if n == 0:
        print(f"  [{name}] hero mask EMPTY"); return
    sizes = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
    blob = lab == (int(np.argmax(sizes)) + 1)
    ring = ndimage.binary_dilation(blob, iterations=6) & ~ndimage.binary_dilation(blob, iterations=1)
    # OUTER ring: past the toon outline, so we measure hero-vs-SCENE not hero-vs-own-ink
    outer = ndimage.binary_dilation(blob, iterations=24) & ~ndimage.binary_dilation(blob, iterations=10)
    hl, bl = L[blob].mean(), L[ring].mean()
    hs, bs = sat[blob].mean() * 100, sat[ring].mean() * 100
    ys, xs = np.where(blob)
    print(f"  [{name}] hero px {blob.sum():6d} ({blob.mean()*100:4.2f}% of frame)  "
          f"bbox y{ys.min()}-{ys.max()} x{xs.min()}-{xs.max()}")
    print(f"           hero L {hl:5.1f}  adjacent-bg L {bl:5.1f}  |dL| {abs(hl-bl):5.1f} "
          f"({abs(hl-bl)/255*100:4.1f}% of range)   hero sat {hs:4.1f}%  bg sat {bs:4.1f}%  dSat {hs-bs:+5.1f}pp")
    ol = L[outer].mean()
    print(f"           vs SCENE beyond the outline (ring 10-24px): bg L {ol:5.1f}  |dL| {abs(hl-ol):5.1f} "
          f"({abs(hl-ol)/255*100:4.1f}% of range)")


print("=" * 78)
print("DEPTH-LAYERING AUDIT — gate3 nohud2 vs golden-beauty-shot-ref")
print("=" * 78)

imgs = [("GOLDEN REF", load(REF), "prop")] + [(k.upper(), load(p), "capsule") for k, p in SHOTS]

print("\n### (1) depth bands: local RMS contrast / saturation / luminance")
for name, im, _ in imgs:
    bands(name, im)

print("\n### (2) vignette / value falloff (edge vs center)")
for name, im, _ in imgs:
    vignette(name, im)

print("\n### (2a) bright-island count — how many separate things compete for the eye")
for name, im, _ in imgs:
    bright_islands(name, im)

print("\n### (2b) toon-outline ink density per depth band")
for name, im, _ in imgs:
    ink_density(name, im)

print("\n### (3) hero silhouette separation vs the pixels touching it")
for name, im, mode in imgs:
    hero_delta(name, im, mode)
print()
