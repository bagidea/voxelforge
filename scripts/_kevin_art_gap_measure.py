#!/usr/bin/env python3
"""Kevin — art-gap vs reference, measurement lane (NO build / NO cargo).

Turns "the art feels flat" into numbers we can chase. Measures REF (CEO's
reference photo) against our two latest plates and prints one table + saves a
comparison figure. Every metric is defined below so a re-run reproduces it.

Metrics (pinned definitions):
  hue        HSV hue 0..360, saturated pixels only (sat >= 0.08, near-grays dropped)
  hue 90%    minimal circular arc (deg) that contains 90% of the saturated hue mass
  occupied   10-deg hue bins holding >= 0.5% of the saturated pixel mass (of 36)
  saturation mean / std over ALL pixels, 0..1
  luminance  Rec.601 luma L = 0.299R+0.587G+0.114B, 0..255
             shadow < 85, midtone 85..170, highlight > 170  (fraction of pixels)
  edge       mean Sobel gradient magnitude per pixel (float gray 0..255) = detail/area
  strong     fraction of pixels with Sobel magnitude > 40
  kmeans     KMeans k=8 on RGB (20k samples, random_state=0); centers as hex + mass
  warm/cool  warm hue in [0,70) or [340,360); cool in [170,270); else neutral.
             ratio = warm mass / cool mass over saturated pixels.
"""
import sys
import numpy as np
from PIL import Image
from scipy import ndimage
from sklearn.cluster import KMeans

REF = r"E:\Projects\bagidea-ai-agents-office\workspace\uploads\1786952280558_773223404_122312179466226047_9178907560287775136_n.jpg"
BEAUTY = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_fl_beauty_20260816\beauty-wide-nohud2.png"
G4 = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_fl_g2g4_20260816\g4-only-nohud2.png"


def rgb(a):
    return np.asarray(a.convert("RGB"), dtype=np.float64)


def hue_sat_val(a):
    """a: HxWx3 float RGB 0..255 -> hue(0..360), sat(0..1), val(0..1)."""
    r, g, b = a[..., 0] / 255.0, a[..., 1] / 255.0, a[..., 2] / 255.0
    mx = np.maximum(np.maximum(r, g), b)
    mn = np.minimum(np.minimum(r, g), b)
    d = mx - mn
    hue = np.zeros_like(mx)
    nz = d > 1e-6
    # red max
    m = nz & (mx == r)
    hue[m] = (60 * ((g[m] - b[m]) / d[m]) % 360)
    m = nz & (mx == g)
    hue[m] = (60 * ((b[m] - r[m]) / d[m]) + 120)
    m = nz & (mx == b)
    hue[m] = (60 * ((r[m] - g[m]) / d[m]) + 240)
    sat = np.zeros_like(mx)
    sat[mx > 1e-6] = d[mx > 1e-6] / mx[mx > 1e-6]
    return hue, sat, mx


def luma(a):
    return 0.299 * a[..., 0] + 0.587 * a[..., 1] + 0.114 * a[..., 2]


def hue_90_range(hue_sat):
    """minimal circular arc containing 90% of saturated hue mass."""
    hue = hue_sat[hue_sat > -1]  # already filtered by caller? use mask instead
    return None


def measure(path, name):
    a = rgb(Image.open(path))
    h, s, v = hue_sat_val(a)
    L = luma(a)

    # --- hue ---
    sat_mask = s >= 0.08
    hs = h[sat_mask]
    h360 = np.zeros(360)
    if hs.size:
        hist, _ = np.histogram(hs, bins=360, range=(0, 360))
        h360 = hist.astype(np.float64) / hs.size
    # occupied 10-deg bins >= 0.5%
    h10 = h360.reshape(36, 10).sum(axis=1)
    occupied = int((h10 >= 0.005).sum())
    # minimal circular arc containing 90% of mass (sliding window)
    if hs.size:
        best = 360
        for start in range(360):
            acc = 0.0
            for k in range(360):
                acc += h360[(start + k) % 360]
                if acc >= 0.90:
                    best = min(best, k + 1)
                    break
        hue90 = best
    else:
        hue90 = 0

    # --- saturation ---
    sat_mean = float(s.mean())
    sat_std = float(s.std())

    # --- luminance zones ---
    shadow = float((L < 85).mean())
    midtone = float(((L >= 85) & (L <= 170)).mean())
    highlight = float((L > 170).mean())
    lum_mean = float(L.mean())

    # --- edge density (Sobel) ---
    gray = ndimage.gaussian_filter(L, sigma=0.8)
    gx = ndimage.sobel(gray, axis=0)
    gy = ndimage.sobel(gray, axis=1)
    mag = np.hypot(gx, gy)
    edge_mean = float(mag.mean())
    strong = float((mag > 40).mean())

    # --- kmeans k=8 ---
    px = a.reshape(-1, 3)
    rng = np.random.default_rng(0)
    n = min(20000, px.shape[0])
    idx = rng.choice(px.shape[0], n, replace=False)
    km = KMeans(n_clusters=8, random_state=0, n_init=10).fit(px[idx])
    centers = km.cluster_centers_
    labs = km.predict(px)
    mass = np.array([(labs == c).mean() for c in range(8)])
    order = np.argsort(-mass)
    centers = centers[order]
    mass = mass[order]
    hexes = ["#%02x%02x%02x" % tuple(np.clip(c, 0, 255).astype(int)) for c in centers]

    # --- warm/cool over saturated pixels ---
    warm = ((h < 70) | (h >= 340)) & sat_mask
    cool = (h >= 170) & (h < 270) & sat_mask
    wmass = float(warm.sum())
    cmass = float(cool.sum())
    warmcool = (wmass / cmass) if cmass > 0 else float("inf")

    return {
        "name": name, "w": a.shape[1], "h": a.shape[0],
        "hue90": hue90, "occupied": occupied,
        "sat_mean": sat_mean, "sat_std": sat_std,
        "shadow": shadow, "midtone": midtone, "highlight": highlight, "lum_mean": lum_mean,
        "edge_mean": edge_mean, "strong": strong,
        "centers": hexes, "mass": mass,
        "warmcool": warmcool,
        "h360": h360, "sat": s, "L": L,
    }


def main():
    rows = [measure(REF, "REF"), measure(BEAUTY, "beauty"), measure(G4, "g4")]
    keys = ["hue90", "occupied", "sat_mean", "sat_std", "shadow", "midtone",
            "highlight", "lum_mean", "edge_mean", "strong", "warmcool"]
    print("=== METRICS ===")
    hdr = "metric".ljust(16) + "".join(r["name"].ljust(14) for r in rows)
    print(hdr)
    for k in keys:
        print(k.ljust(16) + "".join(("%.3f" % r[k]).ljust(14) for r in rows))
    print("\n=== DOMINANT COLORS (k=8) ===")
    for r in rows:
        print(r["name"])
        for hx, m in zip(r["centers"], r["mass"]):
            print("   %s  %.1f%%" % (hx, m * 100))
    import json
    with open("_kevin_art_gap.json", "w") as f:
        json.dump({r["name"]: {k: r[k] for k in keys} for r in rows}, f, indent=2)
    print("\nWROTE _kevin_art_gap.json")
    return rows


if __name__ == "__main__":
    main()
